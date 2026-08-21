# Issue design

## Proposed mechanics

1. **Recording landing (ELR-001).** At rest, after the edition gates are
   green: cut the annotated tag at the rest commit; the next landing appends
   the ledger row citing that commit's full hash. The close guard
   (`EDN-002`'s `git describe --exact-match --match edition-*`) is
   untouched - it proves the tag, not the row. The audit (`EDN-003`) is
   untouched - it compares rows against tags and still refuses absence.
2. **Bootstrap split (ELR-002).** `tools/rtm.ps1 -Channel stable` today
   resolves the ledger from the checkout it builds in, which at the tagged
   commit is stale by construction. The fix separates the two roles: resolve
   and verify ledger/tag agreement from the invoking checkout (the current
   record), then build the tagged commit in a clean separate checkout - a
   linked worktree is sufficient - verifying the build tree is identical to
   the tagged commit before stamping provenance.

This file is incoming evidence. Integrated mechanics remain authoritative
only in the accepted forward authority.
