# Issue design

## Proposed mechanics

The validator turns a failed read of an existing item or gap record into its usual one-line refusal. Nothing else changes.

In [the validator](../../../tools/verify_issue_binding.py), the two `read_record` calls in `def check` (line 290 for the item record, line 320 for each gap record) catch `OSError`. They call `refuse` with `cannot read the item record <path>: <reason>` or `cannot read the residual record <path>: <reason>`, using `shown` for the path as the neighbouring `does not exist` rows do. `def refuse` already prints the prefixed line and exits 1. `def read_or_empty`, `def live_entries`, and every other row keep their behavior.

The same bytes are copied to [the packaged checker](../../../profiles/plan-build/checkers/verify_issue_binding.py). The checker's recorded content digest in [the profile manifest](../../../profiles/plan-build/manifest.toml) is updated in the same change, because `rtm profile` checks every packaged artifact against that digest before it exports. The two new refusal rows are recorded in the forward authority and the new ticket's refusal table; the archived selected issue completion ticket (`t-133`) keeps its bytes.

Tests make a record unreadable without relying on timing. On Windows the test holds the record open with no sharing for the whole validator run, so the existence test still passes and the open fails. Elsewhere it removes read permission; if the file is still readable (for example, the test runs as root), the case reports that it was skipped and why. No test-only code goes into the validator, because the profile ships it.

The mechanics implement [WTC-002](spec.md#requirement-records) and are tested by the [verification plan](test-plan.md#verification).

This file is incoming evidence. Integrated mechanics remain authoritative only in the accepted forward authority.
