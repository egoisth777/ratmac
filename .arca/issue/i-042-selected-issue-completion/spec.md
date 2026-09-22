# Issue specification

## Requirement Records

| Requirement ID | Requirement | Disposition | Rationale | Accepted Forward Authority Refs |
| :--- | :--- | :--- | :--- | :--- |
| `WCP-003` | The self-development workflow binds its selected issue and requirement set to its work item, tested source snapshot, failing-before-implementation evidence, passing-after-implementation evidence, and independent review decision. Completion refuses absent, mismatched, stale, fabricated-by-the-working-agent review authority, or unrelated evidence. A writable success marker, unrelated issue's tests, or a passing child alone cannot satisfy this contract. The parent consumes only the properly bound terminal review result; the Engine remains generic and learns no issue document conventions. | accepted | Accepted under Billy's 2026-09-22 all-wishes instruction and explicit workflow dispatch after independent contract review. | [Accepted authority](../../goal/spec.md#integrated-wishlist-fulfillment-requirements) |

## Evidence and boundaries

The selected wish cites archive tag trial-archive/trial-002-doctor-full-fingerprint and durable log trials/trial-002-doctor-full-fingerprint/trial-log.md as historical provenance, not locally reverified evidence. The current [Plan-Build Runbook](../../../.ratmac/ratmac.toml) ticket class checks red/green receipts but has no independent-review state. [Completion implementation](../../../src/completion.rs) already verifies freshness of declared checks.

Existing files are observed evidence. Proposed checks below have not been executed and are not satisfaction evidence.

## Assumptions

Issue-to-work-item validation belongs to the workflow profile. Reuse generic Run bindings and existing receipt freshness checks. Independent review must use the accepted reviewer-authority boundary being planned from the authorized-review wish; a second agent-writable file is not an authority boundary. The authorization issue i-062 supplies WRA-001: pinned public keys, separate reviewers and designated approver, and exact fresh signed transition intent. A live key generated or accessible by the working agent cannot prove reviewer independence.

## Dependencies

Requires the authorized-review issue i-062 and WRA-001 before this contract can pass. Compose with the review-before-proofs order and declared-data completion; neither substitutes for this issue binding.

## Open decisions

No unresolved disposition remains. The requirement was accepted on 2026-09-22; the linked forward authority governs implementation and its adopted assumptions.
