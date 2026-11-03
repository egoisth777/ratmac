# Wait for the whole pause marker line

```yaml
issue-id: "i-067-whole-pause-marker"
provenance: "Wishlist .arca/wishlist.md:14, \"A private lane that waits on a pause marker reads the whole boundary line\": Billy selected this wish for the next sprint on 2026-10-26."
status: "integrated"
```
## Summary
A test that waits for a pause marker can act on a file whose publisher has created it but not yet filled it. Some waiters return the first read even when it is empty, some accept a partial line, and others move on as soon as the file exists, so a pause-seam test can fail on timing alone; a restored-green rerun of a private turn-tool lane once failed this way and then passed on the same bytes. Every waiter should act only on the marker's whole boundary line and keep waiting, up to its limit, while a read is empty or partial.
Readiness: Ready for planning. Requirements and assumptions are proposed; no acceptance or implementation is claimed.
Provenance: [selected wishlist](../../wishlist.md), line 14 at filing; the exact source entry stays unchanged until its desired end is proved.
Ideal-shape properties advanced: **Every boundary machine-checked; Self-hosted** in [steering](../../steering.md#ideal-shape).
## Routes
| Need | File |
| :--- | :--- |
| Terms | [Ubiquitous language](packs/repos/skills/ratmac/.arca/issue/i-067-whole-pause-marker/ubi-lang.md) |
| Requirements | [Specification](spec.md) |
| Proposed mechanics | [Design](design.md) |
| Verification and integration traces | [Test plan](test-plan.md) |

## Planning decision

Accepted on 2026-10-27 under Billy's 2026-10-26 wish selection after independent
contract review. Advances the steering's Every boundary machine-checked and
Self-hosted properties through its stated outcome. Its open decision is settled
and its accepted assumptions recorded in the forward authority. Integration makes
no implementation claim.
