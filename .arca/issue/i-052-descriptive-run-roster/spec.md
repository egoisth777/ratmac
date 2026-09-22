# Issue specification

## Source and observed gap

`src/cli.rs::roster_line_at` joins `Scheduler::run_roster_at` identifiers with commas. `src/scheduler.rs::ledger_record_of_at` already discovers durable child ownership, and Run Records store State and lifecycle status.

The exact selected wish is **The roster names what each Run is** in [Wishlist](../../wishlist.md), attributed to run-002 postmortem, filed 2026-08-10. Billy's 2026-09-22 Workflow dispatch authorizes preparing this issue; integration was accepted on 2026-09-22.

## Requirement Records

| Requirement ID | Requirement | Disposition | Rationale | Accepted Forward Authority Refs |
| :--- | :--- | :--- | :--- | :--- |
| `WRS-005` | Human-facing Run roster output identifies every canonical address together with its recorded top-level or child role, parent and class for children, recorded binding values when present, and State and lifecycle status when readable. Retired or unreadable records are labeled explicitly rather than silently omitted or guessed. Ordering is stable, metadata comes only from Engine-owned records, and a missing address still refuses instead of implicitly selecting a Run. Listing is read-only and needs no current runbook parse, guard evaluation, or pin rewrite. | accepted | Accepted under Billy's 2026-09-22 all-wishes instruction and explicit workflow dispatch after independent contract review. | [Accepted authority](../../goal/spec.md#integrated-wishlist-fulfillment-requirements) |

## Constraints

Existing [product requirements](../../goal/spec.md) and [working rules](../../schema.md) remain authoritative until explicit integration. Historical records keep their bytes. The Engine alone writes runtime state; a contributor's claim never authorizes motion or proves completion.

## Dependencies

No new hard dependency. Shares read-only historical-record handling with terminal-history status; successor publication must not expose an ownership-free row.
