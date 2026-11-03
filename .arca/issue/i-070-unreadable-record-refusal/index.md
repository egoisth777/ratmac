# Refuse an unreadable workflow record by name

```yaml
issue-id: "i-070-unreadable-record-refusal"
provenance: "Wishlist .arca/wishlist.md:15, \"The issue-binding validator refuses an unreadable record by name instead of crashing\"; Billy selected this wish for the next sprint on 2026-10-26."
status: "integrated"
```

## Summary

When the selected-issue binding validator, `python tools/verify_issue_binding.py check`, reaches the item record its turn is bound to, or one of that item's gap records, and the record exists but cannot be opened, it prints a Python traceback instead of refusing with one named line like every other refusal. A guard or caller that branches on that one-line refusal gets a multi-line crash. The validator, and its copy packaged in the plan-build profile, should refuse such a record by name with exit 1 and write nothing.

Readiness: Ready for planning. Requirements and assumptions are proposed; no acceptance or implementation is claimed.

Provenance: [selected wishlist](../../wishlist.md), line 15 at filing; the exact source entry stays unchanged until its desired end is proved.

Ideal-shape properties advanced: **Refusals are branchable and Every boundary machine-checked** in [steering](../../steering.md#ideal-shape).

## Routes

| Need | File |
| :--- | :--- |
| Terms | [Ubiquitous language](packs/repos/skills/ratmac/.arca/issue/i-070-unreadable-record-refusal/ubi-lang.md) |
| Requirements | [Specification](spec.md) |
| Proposed mechanics | [Design](design.md) |
| Verification and integration traces | [Test plan](test-plan.md) |

## Planning decision

Accepted on 2026-10-27 under Billy's 2026-10-26 wish selection after independent
contract review. Advances the steering's Refusals are branchable and Every boundary
machine-checked properties through its stated outcome. The choices it leaves to the
ticket stay there; its accepted assumptions are recorded in the forward authority.
Integration makes no implementation claim.
