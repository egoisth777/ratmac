# Issue specification

## Requirement Records

| Requirement ID | Requirement | Disposition | Rationale | Accepted Forward Authority Refs |
| :--- | :--- | :--- | :--- | :--- |
| `WCP-008` | The supported edition-cut path validates the candidate annotation using the same proof-text contract as the edition audit, confirms the exact intended clean commit and all required successful checks against its bytes, validates edition naming/sequence and target reachability, and proves the edition rule EDN-001 rest condition at that commit before creating a tag: no pending intake, no open work item, and no unproven gap record. A missing proof phrase, failed or stale check, unfinished cycle despite green quality commands, dirty tree, moved target, existing name, or malformed input creates no tag and alters no existing tag. A valid cut creates one immutable annotated tag using the validated text and exact checked commit; ledger recording still follows tag creation. Text that merely names checks is not proof that they passed. | accepted | Accepted under Billy's 2026-09-22 all-wishes instruction and explicit workflow dispatch after independent contract review. | [Accepted authority](../../schema.md#wcp-008--edition-proof-preflight) |

## Evidence and boundaries

[Edition audit](../../../test/qa/src/edition.rs) defines RECORDED_BAR and audits existing tags only; its module contract says every operation is read-only. [Edition rules](../../schema.md#editions) require immutable tags and later ledger recording. The wishlist's edition-006 incident supplies historical provenance.

Existing files are observed evidence. Proposed checks below have not been executed and are not satisfaction evidence.

## Assumptions

Expose a narrow contributor edition-cut operation reusing the Rust edition-audit functions and existing declared quality checks. Its check-only form makes the candidate reviewable. This issue does not authorize creating a tag during planning, bypassing repository authorization, moving an existing edition, or fetching a remote.

## Dependencies

Contributor-tools lane supplies the carrier and evidence standard. Reuse existing edition audit, immutable-edition, and ledger-order requirements.

## Open decisions

No unresolved disposition remains. The requirement was accepted on 2026-09-22; the linked forward authority governs implementation and its adopted assumptions.
