# Issue test plan

## Verification

These are proposed checks, not executed results. Each check references the observable contract in [the specification](spec.md#requirement-records).

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| Disabled behavior (`WEBV-025`) | `WEB-007` | With the variable absent and with 0, compare stdout, required stderr, exit status, and filesystem before/after against baseline. An allocation-counting unit test proves a disabled event does not evaluate allocating field expressions. |
| Structured decisions (`WEBV-026`) | `WEB-007` | With 1, controlled fixtures exercise root resolution, edge choice, refusal, root-before-Run lock order, and mint decisions. Records have stable codes/ordering and appear only on stderr; ordinary stdout and state match disabled execution. |
| Determinism and privacy (`WEBV-027`) | `WEB-007` | Run equivalent isolated fixtures under different absolute paths and clocks; structured trace records match byte for byte. Quotes/newlines escape correctly; no timestamp, duration, absolute path, process ID, secret-bearing environment value, or arbitrary child diagnostic appears in those records. Separately assert that mandatory errors keep their established actionable text and paths. |
| Single owner and no proof dependency (`WEBV-028`) | `WEB-007` | An exhaustive source check finds stderr emission only in the owning module. Invalid toggle values and stderr failure are handled as specified. Existing fault-point lanes still work, and guards/receipts/hidden behavioral lanes do not parse trace text. |

## Goal/Test File Traces

Integration was accepted on 2026-09-22; the table below records the authority changes. Product goal/design/test list, stable diagnostic-code ownership, operator guidance, and working evidence rules need integration.

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
| `AGENTS.md` | unaffected | No change in this intake bundle. |
| `.arca/schema.md` | unaffected | [Accepted authority](../../goal/spec.md#integrated-wishlist-fulfillment-requirements). |
| `.arca/runbook-spec.md` | unaffected | Integration must update this authority if the accepted design adds or changes declared data. |
