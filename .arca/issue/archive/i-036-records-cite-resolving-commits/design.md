# Issue design

## Proposed mechanics

1. **Stamp landing (RCR-001).** P5's fixed order ends "kills into the owning
   gap record, `git commit --amend` into the green landing"
   (`.arca/schema.md:265`): the record's bytes exist before the amend
   finalizes the landing, so any hash written during P5 predicts. The fix is
   ELR-001's move. The green landing merges to `main`; a following stamp
   landing flips the record `satisfied` and writes `implementation-revision`
   by resolving the landed tip at stamp time. The 2026-08-21 sprint already
   made separate stamp landings physically (`e29aea0`, `ee8a457`, `9a855ae`,
   `bca30d4`, each "stamp landed-commit; log the landing") — the defect is
   only that the stamps carried the worktree hash first written inside the
   green landing (`git show 6cee4bb:.arca/residual/res-151.md` already reads
   `git:7fa79f6`) instead of deriving it after the merge. The ticket's
   `landed-commit` (`.arca/tpl/ticket.md:27-29`) is stamped in the same
   landing.

2. **The resolving-citation check (RCR-002).** One checker walks every gap
   record across the active folder and the archive — the namespace the
   record contract already counts for presence and uniqueness
   (`.arca/schema.md:730-731`) — extracts the `git:` hash from
   `implementation-revision`, and demands two answers: the object exists
   (`git cat-file -e`), and it is reachable from a ref (ancestor of `HEAD`
   or held by a tag). Existence without reachability is the observed
   failure: all four 2026-08-21 hashes answer `cat-file` as commits while
   `git for-each-ref --contains` names none and `git fsck --dangling` lists
   them (`7fa79f691423d6662a33ce221ca8f752b47bb717`,
   `8cfcdec07a385c448226b26eaa0e5f20340f1437`,
   `b9163fd189cfa8384eb7b244c71777c65149582a`,
   `17f3c753188f3232ab3d8c0b6c123c7a577ed25f`). The engine gate never reads
   the field today (`src/contract.rs:680-695`), so this is new: the qa crate
   walking this repository is the proposed carrier, and P1 may move it into
   the engine's `record_contract` instead.

3. **Historical carriers (the RCR-002/RCR-003 boundary).** Archived records
   keep their bytes (SVC-008), and 11 of them cite unreachable commits: the
   seven 2026-08-21 records, `res-116`/`res-117` (`git:0231def`, labeled "the
   t-083 green tree" while the landed `b1025ff` resolves), and
   `res-124`/`res-125` (`git:0770778`). The SVC-008/t-084 pattern carries
   them: one enumerated allowlist, each row naming the record, the hash, and
   the reason it predates the rule; a row matching nothing fails as stale.
   Post-rule records get no allowlist path — the archive move is where a new
   citation is proved.

4. **Mechanical re-point (RCR-003).** The stamp step is one re-runnable
   action: after any amend, reset, checkpoint fold, or branch delete moves a
   cited commit, re-running it re-derives the record's citation and the
   ticket's `landed-commit` from the repository in the same change. The
   archive-rules sentence "a record carrying a pending obligation (e.g. a
   commit-hash re-stamp) may archive; the obligation travels with the file"
   (`.arca/schema.md:400-401`) keeps its meaning and gains a discharge proof
   — the obligation is discharged only when the check passes. The 75
   archived tickets' `landed-commit` values all resolve at filing; the
   record side is where the hand discipline leaked.

## Open decisions for P1

- Carrier: qa checker over this repository, or engine `record_contract`
  gate (the engine already resolves git identity for pinning).
- Reachability binds `HEAD` only, or any ref. Proposed: any ref — an edition
  tag holds its commit reachable by design (`.arca/schema.md:539-542`), and
  a record citing a tagged edition commit must not fail for not being on
  `HEAD`'s first-parent line.

This file is incoming evidence. Integrated mechanics remain authoritative
only in the accepted forward authority.
