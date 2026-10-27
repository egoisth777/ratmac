# Keep a hold barrier inside its own test case

```yaml
issue-id: "i-068-case-scoped-hold-barrier"
provenance: "Wishlist .arca/wishlist.md:20, \"A hold barrier that one cycle-pause-routes test case sets never reaches another case's `rtm hold`, and a check proves it\": Billy selected this wish for the next sprint on 2026-10-26."
status: "integrated"
```
## Summary
One case of the cycle-pause-routes suite arms the Engine's hold test barrier through process-wide settings, while the suite's command helper runs its cases' Engine commands inside that same test process. Other cases' `rtm hold` can therefore wait on that barrier and fail when it expires, even though their own fixtures are fine. A test hook a case arms should reach only that case's own commands, and a check should prove that a case running alongside never sees it.
Readiness: Ready for planning. Requirements and assumptions are proposed; no acceptance or implementation is claimed.
Provenance: [selected wishlist](../../wishlist.md), line 20 at filing; the exact source entry stays unchanged until its desired end is proved.
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
