# Keep every test scratch folder unique

```yaml
issue-id: "i-066-unique-test-scratch"
provenance: "Wishlist .arca/wishlist.md:13, \"Every test's scratch folder stays unique even when two tests start in the same clock tick\": Billy selected this wish for the next sprint on 2026-10-26."
status: "integrated"
```
## Summary
Most public test suites that make a scratch folder name it from the process id and the clock, and the Windows clock advances in 100-nanosecond steps, so two tests that start in the same step can be handed one folder and fail with each other's damage while the same suite passes alone. Every test should instead get a folder no other test can be handed, from one shared helper that creates it new, does not rest on the clock alone, and retries a taken name, so a suite gives the same verdict in parallel as alone.
Readiness: Ready for planning. Requirements and assumptions are proposed; no acceptance or implementation is claimed.
Provenance: [selected wishlist](../../wishlist.md), line 13 at filing; the exact source entry stays unchanged until its desired end is proved.
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
