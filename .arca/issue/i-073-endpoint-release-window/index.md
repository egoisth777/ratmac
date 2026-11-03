# Prove a resumed child claims its lock after a release window

```yaml
issue-id: "i-073-endpoint-release-window"
provenance: "Wishlist .arca/wishlist.md:19, \"A resumed endpoint child that meets a release window at its Run lock still claims the lock once the window closes, and a test proves it\"; Billy selected this wish for the next sprint on 2026-10-26."
status: "integrated"
```

## Summary

A resumed child Engine endpoint takes the child Run's lock twice: once to claim it, and once for its terminal read. Both already retry when they meet a peer's release window, but no test puts a release window at either place. So the deliberate damage that made the single lock try report a release window as not acquired survived every public and private check of the lock-wait ticket. The endpoint should keep waiting out a release window and claiming or reading once it closes, and a named check should fail whenever the window is treated as a held lock.

Readiness: Ready for planning. Requirements and assumptions are proposed; no acceptance or implementation is claimed.

Provenance: [selected wishlist](../../wishlist.md), line 19 at filing; the exact source entry stays unchanged until its desired end is proved.

Ideal-shape properties advanced: **Every boundary machine-checked and One writer, append-only** in [steering](../../steering.md#ideal-shape).

## Routes

| Need | File |
| :--- | :--- |
| Terms | [Ubiquitous language](packs/repos/skills/ratmac/.arca/issue/i-073-endpoint-release-window/ubi-lang.md) |
| Requirements | [Specification](spec.md) |
| Proposed mechanics | [Design](design.md) |
| Verification and integration traces | [Test plan](test-plan.md) |

## Planning decision

Accepted on 2026-10-27 under Billy's 2026-10-26 wish selection after independent
contract review. Advances the steering's Every boundary machine-checked and One
writer, append-only properties through its stated outcome. The choices it leaves to
the ticket stay there; its accepted assumptions are recorded in the forward
authority. Integration makes no implementation claim.
