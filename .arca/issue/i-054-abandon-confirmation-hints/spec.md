# Issue specification

## Source and observed gap

`src/abandon.rs` documents an addressed command with a project-name phrase. Its `plan_abandon` checks `required_phrase` before determining whether an omitted address is invalid because Runs exist; `src/cli.rs::abandon` also renders option errors before all arguments have necessarily been parsed. The explicit help text already distinguishes Run retirement from leftover-lock cleanup.

The exact selected wish is **The abandon usage hint names the real confirmation phrase** in [Wishlist](../../wishlist.md), attributed to Advisor, 2026-09-14. Billy's 2026-09-22 Workflow dispatch authorizes preparing this issue; integration was accepted on 2026-09-22.

## Requirement Records

| Requirement ID | Requirement | Disposition | Rationale | Accepted Forward Authority Refs |
| :--- | :--- | :--- | :--- | :--- |
| `WRS-007` | All live abandonment hints, examples, and malformed-option diagnostics distinguish addressed retirement from unaddressed leftover-lock cleanup. For an addressed Run they give the exact `abandon <run id>` phrase independent of option order. When an address is missing while admitted Runs exist, the diagnostic first requires `--run <id>`, shows the roster, and teaches an addressed example rather than offering a project-name phrase that cannot retire those Runs. Only a true no-admitted-Run leftover-lock path teaches the project-name phrase. No diagnostic performs retirement or chooses a Run implicitly; confirmation and refusal safety remain unchanged. | accepted | Accepted under Billy's 2026-09-22 all-wishes instruction and explicit workflow dispatch after independent contract review. | [Accepted authority](../../goal/spec.md#integrated-wishlist-fulfillment-requirements) |

## Constraints

Existing [product requirements](../../goal/spec.md) and [working rules](../../schema.md) remain authoritative until explicit integration. Historical records keep their bytes. The Engine alone writes runtime state; a contributor's claim never authorizes motion or proves completion.

## Dependencies

Independent. Reuse the descriptive roster renderer if it lands first, without requiring that presentation change to fix the phrase.
