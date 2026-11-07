# Let abandon read a lock through a release window

```yaml
issue-id: "i-072-abandon-release-window"
provenance: "Wishlist .arca/wishlist.md:17, \"`rtm abandon` never refuses only because another command is partway through releasing a lock it is checking\"; Billy selected this wish for the next sprint on 2026-10-26."
status: "integrated"
```

## Summary

While holding no lock, `rtm abandon` checks whether the project's root lock and the addressed Run's lock exist, and it refuses on any read error other than "not found". On Windows a lock that another command is releasing stays delete-pending until that command's handle closes, and a read taken just as the handle closes can answer `Access is denied. (os error 5)`. So abandon can refuse only because a release happened to be in progress. Abandon should wait out the release within the lock module's wait bound and use the settled answer, refusing with today's message only for a lasting or different failure.

Readiness: Ready for planning. Requirements and assumptions are proposed; no acceptance or implementation is claimed.

Provenance: [selected wishlist](../../wishlist.md), line 17 at filing; the exact source entry stays unchanged until its desired end is proved.

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
append-only properties through its stated outcome. The choices it leaves to the
ticket stay there; its accepted assumptions are recorded in the forward authority.
Integration makes no implementation claim.
