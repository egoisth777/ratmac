# Finish the skill move through a brief file hold

```yaml
issue-id: "i-071-skill-move-retry"
provenance: "Wishlist .arca/wishlist.md:16, \"`rtm skill` finishes the move into place even when another program briefly holds a just-written file open\"; Billy selected this wish for the next sprint on 2026-10-26."
status: "integrated"
```

## Summary

`rtm skill <folder>` writes a complete hidden partial folder and moves it into place with a single try. On Windows that move sometimes fails with `Access is denied. (os error 5)`, likely, though not yet proven, because another program such as a virus scanner or the search indexer briefly holds a just-written file open; the command then refuses, and full test runs fail by chance. The move should wait out such a brief hold within the lock module's wait bound and succeed, while a lasting or different failure still refuses with today's message. Because the move now retries, each attempt first checks that nothing has appeared at the requested path.

Readiness: Ready for planning. Requirements and assumptions are proposed; no acceptance or implementation is claimed.

Provenance: [selected wishlist](../../wishlist.md), line 16 at filing; the exact source entry stays unchanged until its desired end is proved.

Ideal-shape properties advanced: **Refusals are branchable and Self-hosted** in [steering](../../steering.md#ideal-shape).

## Routes

| Need | File |
| :--- | :--- |
| Terms | [Ubiquitous language](packs/repos/skills/ratmac/.arca/issue/i-071-skill-move-retry/ubi-lang.md) |
| Requirements | [Specification](spec.md) |
| Proposed mechanics | [Design](design.md) |
| Verification and integration traces | [Test plan](test-plan.md) |

## Planning decision

Accepted on 2026-10-27 under Billy's 2026-10-26 wish selection after independent
contract review. Advances the steering's Refusals are branchable and Self-hosted
properties through its stated outcome. The choices it leaves to the ticket stay
there; its accepted assumptions are recorded in the forward authority. Integration
makes no implementation claim.
