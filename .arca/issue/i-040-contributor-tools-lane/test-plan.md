# Issue test plan

## Verification

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| `WCPV-001` | [WCP-001](spec.md#requirement-records) | Guidance-consistency evidence: manually review the declared executable-tool ownership and issue-to-ticket carrier against tools/turn.ps1, tools/check_links.py, source, and tests. Each executable behavior has an explicit lane, owner, carrier, and required evidence standard in the working guidance. No executable deliverable is created for this policy-only requirement. |
| `WCPV-002` | [WCP-001](spec.md#requirement-records) | Guidance-consistency evidence: manually inspect the complete lane text and representative issue/ticket records. A documentation-only change stays in the shop lane, while a script or helper changing executable behavior cannot claim that exception. |
| `WCPV-003` | [WCP-001](spec.md#requirement-records) | Guidance-consistency evidence: manually inspect the archived ticketed turn-lifecycle evidence to confirm it demonstrates the required failing-before-implementation and passing-after-implementation rule. Cite the existing ticket and its recorded proof; do not create, change, or rerun a tool or test solely to satisfy this policy-only requirement. This review is not fresh behavioral proof. |

These are planned checks, not results. Executable work enters its approved ticket before tests or implementation are authored.

## Goal/Test File Traces

| Goal/Test File | Status | Reverse Issue Refs |
| :--- | :--- | :--- |
| `.arca/goal/index.md` | updated | [Accepted authority](../../schema.md#wcp-001--contributor-tools-lane). |
| `.arca/goal/ubi-lang.md` | updated | [Accepted authority](../../schema.md#wcp-001--contributor-tools-lane). |
| `.arca/goal/spec.md` | unaffected | [Accepted authority](../../schema.md#wcp-001--contributor-tools-lane). |
| `.arca/goal/design.md` | updated | [Accepted authority](../../schema.md#wcp-001--contributor-tools-lane). |
| `.arca/goal/test-list.md` | updated | [Accepted authority](../../schema.md#wcp-001--contributor-tools-lane). |

## Contributor Authority/Schema Traces

| Authority or Schema Artifact | Status | Integration and Reverse Refs |
| :--- | :--- | :--- |
| `AGENTS.md` | unaffected | No change during filing. |
| `.arca/schema.md` | updated | [Accepted authority](../../schema.md#wcp-001--contributor-tools-lane). |
| `.arca/wishlist.md` | unaffected | Original wish retained unchanged until its complete desired end is proved. |
| `.ratmac/ratmac.toml` | unaffected | No active Run contract is edited during filing. |
