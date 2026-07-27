# Issue authoring has no scaffold and no enforceable shape

```yaml
issue-id: "i-011-issue-authoring-scaffold"
provenance: "trial-001-issue-authoring, trials/trial-001-issue-authoring/trial-log.md"
status: "pending"
```

## Summary

Creating an issue is the entry point of the whole loop, and it is the only
step with neither a scaffold nor a usable check. A contributor hand-copies
five blanks, hand-derives an issue number with no stated rule, and hand-fills
30 placeholders. Nothing verifies the result: the contributor check refuses a
legitimately `pending` issue outright, and the PGE-001 intake gate accepts a
folder that is nothing but unfilled blanks. Three of the four front-matter
rules stated in `.arca/index.md` are enforced by nothing. The blanks also ask
the author for `Disposition` and requirement IDs, which `.arca/index.md`
assigns to P1, so an honest author cannot fill them.

## Routes

| Need | File |
| :--- | :--- |
| Terms | [Ubiquitous language](ubi-lang.md) |
| Requirements | [Specification](spec.md) |
| Proposed mechanics | [Design](design.md) |
| Verification and integration traces | [Test plan](test-plan.md) |
