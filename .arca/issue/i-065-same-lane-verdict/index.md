# Give a private lane one verdict everywhere

```yaml
issue-id: "i-065-same-lane-verdict"
provenance: "Wishlist .arca/wishlist.md:12, \"A private lane gives the same verdict inside the full workspace proof as it does alone\": Billy selected this wish for the next sprint on 2026-10-26."
status: "integrated"
```

## Summary

A private lane (a crate under `test-hidden/`) can read red inside the full workspace proof or a lane sweep and then pass when run alone with nothing changed, so a red verdict cannot be trusted. In the recorded cases the single-Engine-binary lane rebuilds, hashes, and runs the Engine in the repository's shared build folder while other tests may rewrite it; the wish says this cause is inferred, not proven. No lane test should read, run, or rebuild build output, folders, or settings that something running at the same time can write. This does not by itself prove the wish's desired end; wall-clock limits are the next suspect, and the wish stays open.

Readiness: Ready for planning. Requirements and assumptions are proposed; no acceptance or implementation is claimed.

Provenance: [selected wishlist](../../wishlist.md), line 12 at filing; the exact source entry stays unchanged until its desired end is proved.

Ideal-shape properties advanced: **Every boundary machine-checked; Self-hosted** in [steering](../../steering.md#ideal-shape).

## Routes

| Need | File |
| :--- | :--- |
| Terms | [Ubiquitous language](ubi-lang.md) |
| Requirements | [Specification](spec.md) |
| Proposed mechanics | [Design](design.md) |
| Verification and integration traces | [Test plan](test-plan.md) |

## Planning decision

Accepted on 2026-10-27 under Billy's 2026-10-26 wish selection after independent
contract review. Advances the steering's Every boundary machine-checked and
Self-hosted properties through its stated outcome. Its open decisions are settled
and its accepted assumptions recorded in the forward authority. Integration makes
no implementation claim.
