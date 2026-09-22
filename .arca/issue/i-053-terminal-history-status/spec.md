# Issue specification

## Source and observed gap

The wishlist names passed run-020 and run-023, whose status refused after runbook edits and whose subsequent retirement marked an already-passed child abandoned. `src/cli.rs::status` opens a Scheduler before reporting; `src/scheduler.rs::status` loads the current class and verifies its hash before rendering the Run.

The exact selected wish is **A rested Run stays readable after the runbook moves on** in [Wishlist](../../wishlist.md), attributed to Advisor, 2026-09-14. Billy's 2026-09-22 Workflow dispatch authorizes preparing this issue; integration was accepted on 2026-09-22.

## Requirement Records

| Requirement ID | Requirement | Disposition | Rationale | Accepted Forward Authority Refs |
| :--- | :--- | :--- | :--- | :--- |
| `WRS-006` | Addressed status can report a strictly parsed persisted passed Run's identity, State, status, and recorded evidence identities even when the current runbook differs, is unavailable, or cannot parse. This historical view reports that current instructions are unavailable and never renders changed prompts or guards as the old Run's instructions. It neither evaluates guards nor mutates records, receipts, pins, ledgers, history, or locks, and never suggests retirement merely to read a completed Run. Nonterminal motion and reporting retain their existing pin checks; corrupted records still refuse by name. | accepted | Accepted under Billy's 2026-09-22 all-wishes instruction and explicit workflow dispatch after independent contract review. | [Accepted authority](../../goal/spec.md#integrated-wishlist-fulfillment-requirements) |

## Constraints

Existing [product requirements](../../goal/spec.md) and [working rules](../../schema.md) remain authoritative until explicit integration. Historical records keep their bytes. The Engine alone writes runtime state; a contributor's claim never authorizes motion or proves completion.

## Dependencies

Independent from rule-fix recovery: reading completed facts authorizes no new motion. Coordinate roster formatting for consistent role and uncertainty labels.
