# Run the test Engine Cargo just built

```yaml
issue-id: "i-063-build-message-test-engine"
provenance: "Wishlist .arca/wishlist.md:11, \"Resolve the Engine executable from Cargo's own build messages\": Billy selected this wish for the next sprint on 2026-10-26."
status: "integrated"
```

## Summary

Test helpers that build a binary from this repository and then run it guess where the build put it: the shared helper `today_engine()` builds the test Engine `rtm-qa` and then runs whatever sits at `target/debug/rtm-qa` under the repository. When Cargo's build output goes elsewhere, the fresh build is not what runs: an older binary left in that folder runs instead, or the helper stops because nothing is there, and private lanes each carry their own workaround. Every such helper should run the executable Cargo reports for the build it just made, and the private lanes should share that one answer.

Readiness: Ready for planning. Requirements and assumptions are proposed; no acceptance or implementation is claimed.

Provenance: [selected wishlist](../../wishlist.md), line 11 at filing; the exact source entry stays unchanged until its desired end is proved.

Ideal-shape properties advanced: **Self-hosted; Every boundary machine-checked** in [steering](../../steering.md#ideal-shape).

## Routes

| Need | File |
| :--- | :--- |
| Terms | [Ubiquitous language](ubi-lang.md) |
| Requirements | [Specification](spec.md) |
| Proposed mechanics | [Design](design.md) |
| Verification and integration traces | [Test plan](test-plan.md) |

## Planning decision

Accepted on 2026-10-27 under Billy's 2026-10-26 wish selection after independent
contract review. Advances the steering's Self-hosted and Every boundary
machine-checked properties through its stated outcome. Its open decisions are settled
and its accepted assumptions recorded in the forward authority. Integration makes
no implementation claim.
