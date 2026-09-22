# Issue specification

## Requirement Records

| Requirement ID | Requirement | Disposition | Rationale | Accepted Forward Authority Refs |
| :--- | :--- | :--- | :--- | :--- |
| `WCP-007` | A distributable Plan-Build profile contains the Machine Class template, executable artifact schemas and validators, issue/gap/work-item blanks, required terms, and operating instructions needed by its guards. A clean repository with no .arca/schema.md and no inherited system prompt can initialize the profile and drive a complete issue-to-proven-completion sprint using only its packaged contract and Engine output. Profile fields and roots are declared data; the Engine gains no project-specific source code. Missing, malformed, or incompatible profile artifacts refuse with actionable guidance. Existing user files are never overwritten silently, and installation is deterministic and offline. | accepted | Accepted under Billy's 2026-09-22 all-wishes instruction and explicit workflow dispatch after independent contract review. | [Accepted authority](../../goal/spec.md#integrated-wishlist-fulfillment-requirements) |

## Evidence and boundaries

[Operator skill writer](../../../src/skill.rs) exports generic operating guidance. The [Plan-Build Runbook](../../../.ratmac/ratmac.toml) and [issue/ticket templates](../../tpl/issue/index.md) currently rely on project workflow rules. [Contract implementation](../../../src/contract.rs) validates record shapes; this work must make those shapes an explicit packaged contract, not just copy prose.

Existing files are observed evidence. Proposed checks below have not been executed and are not satisfaction evidence.

## Assumptions

Package one versioned profile alongside the existing distributable operator skill, reusing its atomic non-overwriting export shape, but keep the profile's artifact contract distinct from generic Engine operation. Initialization explicitly chooses a target and declared roots; it does not install global configuration or dependencies.

## Dependencies

Depends on the selected-issue completion and reviewer authority contracts for a full proven sprint; incorporate contributor-tools ownership for packaged validators. Preserve generic Engine/working-authority boundaries.

## Open decisions

No unresolved disposition remains. The requirement was accepted on 2026-09-22; the linked forward authority governs implementation and its adopted assumptions.
