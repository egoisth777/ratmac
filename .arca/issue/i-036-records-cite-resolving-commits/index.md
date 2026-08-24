# A gap record cites only commits that resolve

```yaml
issue-id: "i-036-records-cite-resolving-commits"
provenance: "Wishlist, 'A gap record must never cite a commit that no longer resolves' — Billy's shop, filed 2026-08-10; fresh evidence from the 2026-08-21 sprint close: res-147..res-153 cite four 'green landing' hashes that survive only as dangling objects"
status: "pending"
```

## Summary

A gap record's `implementation-revision` is written before the landing it
cites is final, so it names a hash that an amend, a reset, or a checkpoint
fold then orphans. The wish filed on 2026-08-10 recorded four hand
corrections; the 2026-08-21 sprint reproduced the defect on all four of its
tickets at once: `res-147`..`res-148` cite `git:17f3c75`, `res-149`..`res-150`
cite `git:8cfcdec`, `res-151` cites `git:7fa79f6`, and `res-152`..`res-153`
cite `git:b9163fd` — each labeled "the green landing", and each now held only
as a dangling commit (`git cat-file -t` answers `commit`; no ref contains it;
`git fsck --dangling` lists all four). The mechanism is the same
self-reference i-034 retired for the edition ledger: `res-151` already cited
`git:7fa79f6` inside the merged green landing `6cee4bb` itself, so the
citation predicted a hash that could never become the commit containing it —
P5's own order mandates this, writing the record's bytes before the
`git commit --amend` that finalizes the green landing.

Nothing checks the field: `src/contract.rs` validates the
`frozen-goal-bundle-revision` citation and never reads `implementation-revision`
anywhere in `src/`. The current control is hand re-checking, and it rots too —
a full scan at filing finds 11 of 153 records citing unreachable commits:
the seven 2026-08-21 records, plus `res-116`/`res-117` (`git:0231def`) and
`res-124`/`res-125` (`git:0770778`), two rot events no hand pass ever caught.

This issue proposes the ELR-001 move for gap records — the stamp follows the
landing it cites — plus a machine check that refuses an unresolvable
citation, so the stamp is proved at closure instead of remembered.

## History

- 2026-08-24: filed from the wishlist by the DESIGNER; dispositions are the
  author's proposal, P1 confirms or revises at integration. The wish's own
  record ids for the four hand corrections (`res-121`, `res-127`, `res-128`,
  `res-134`) do not match the log's stamp events; this bundle cites the
  verified events at `.arca/log.md:300`, `:342`, `:344`, and `:410`.

## Routes

| Need | File |
| :--- | :--- |
| Terms | [Ubiquitous language](ubi-lang.md) |
| Requirements | [Specification](spec.md) |
| Proposed mechanics | [Design](design.md) |
| Verification and integration traces | [Test plan](test-plan.md) |
