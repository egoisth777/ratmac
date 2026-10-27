# Refuse a repeated run address on every command

```yaml
issue-id: "i-069-repeated-run-refusal"
provenance: "Wishlist .arca/wishlist.md:9, \"Every Run-addressing command refuses a repeated `--run`\"; Billy selected this wish for the next sprint on 2026-10-26."
status: "integrated"
```

## Summary

`rtm hold`, `rtm resume`, `rtm recover`, `rtm spawn`, and `rtm respawn` accept `--run` twice and quietly act on the last value. `rtm status`, `rtm step`, `rtm intent`, and `rtm abandon` refuse the same input and print the Run roster. So a pasted or mistyped command line can hold, resume, recover, spawn under, or supersede a Run the caller did not mean. Every Run-addressing command should refuse a repeated `--run` the same way, print the roster, and change nothing.

Readiness: Ready for planning. Requirements and assumptions are proposed; no acceptance or implementation is claimed.

Provenance: [selected wishlist](../../wishlist.md), line 9 at filing; the exact source entry stays unchanged until its desired end is proved.

Ideal-shape properties advanced: **Refusals are branchable and One writer, append-only** in [steering](../../steering.md#ideal-shape).

## Routes

| Need | File |
| :--- | :--- |
| Terms | [Ubiquitous language](ubi-lang.md) |
| Requirements | [Specification](spec.md) |
| Proposed mechanics | [Design](design.md) |
| Verification and integration traces | [Test plan](test-plan.md) |

## Planning decision

Accepted on 2026-10-27 under Billy's 2026-10-26 wish selection after independent
contract review. Advances the steering's Refusals are branchable and One writer,
append-only properties through its stated outcome. The choice it leaves to the
ticket stays there; its accepted assumptions are recorded in the forward authority.
Integration makes no implementation claim.
