# Issue specification

Dispositions below record the author's proposed decision; P1 confirms or revises them at
integration. `EDN` expands to **Edition** and is this issue's stable requirement-ID prefix,
defined in [ubi-lang.md](ubi-lang.md).

All three asks bind contributors and this repository's own Machine Class, not the engine
program: no goal row is proposed, and each accepted ask resolves to a requirement-ID heading
in the working authority. Billy settled the three open choices - the name, the strength of the
enforcement, and the bar - on 2026-08-10 before the bundle was written, so all three are
proposed `accepted`.

## Requirement Records

| Requirement ID | Requirement | Disposition | Rationale | Accepted Forward Authority Refs |
| :--- | :--- | :--- | :--- | :--- |
| `EDN-001` | An **edition** is the shop's only marker for "this commit is a stable base to develop the engine from": an annotated tag named `edition-NNN`, sequential from `edition-001`, whose message records what was proven. It may be cut only where the cycle is at rest - nothing pending in the intake work area, no open work item, no gap record left unproven - with the full workspace suite, formatting, lints, and the link check green, `rtm doctor` on this repository exiting `0`, and the working tree clean and identical to that commit. | accepted | Self-development makes the base a correctness input: work built on a commit that was never green is unfalsifiable, and today the only way to ask "is this a good tree" is to re-run every gate by hand. `checkpoint` cannot be reused - the working rules already spend that word on the throwaway safety commit inside a ticket worktree - so the marker needs its own name. Billy chose `edition-NNN` over semver, which would imply a published artifact and a compatibility promise this shop does not make. | |
| `EDN-002` | The cycle refuses to reach rest without one. The `close` stage of this repository's Machine Class carries an Exit Guard that passes only when the commit being left is exactly an edition, so no sprint can finish unmarked - and therefore every sprint necessarily starts from an edition. The check is spelled in the existing closed guard vocabulary and adds no guard kind and no engine behavior. | accepted | A marker nobody checks rots, and this shop's standing habit is to convert a remembered ritual into a machine check. Putting it on `close` needs no new concept: an Exit Guard already decides whether a stage may be left, the probe is one read-only version-control command, and the ordering is the natural one - the landings are committed, the edition is cut, the step into rest then passes. | |
| `EDN-003` | An edition, once cut, is never moved and never deleted, and the sequence takes the next unused number even if an edition is later judged bad; a bad edition is retired by cutting the next one, not by rewriting the last. | accepted | Editions are what make a cited commit keep resolving: a tag holds its commit reachable, so a gap record citing an edition cannot rot the way a bare hash on an unmerged line can. That property is worth nothing if the tag can move under a citation, and it directly strengthens the live wish about gap records citing commits that no longer resolve. | |

## Out of scope

- **No engine behavior change.** No new guard kind, no version-control knowledge inside the
  engine, no change to routing, receipts, locking, exit codes, or the Run Record. The engine
  keeps knowing nothing about tags; only this repository's runbook and working rules change.
- **No automation for cutting one.** Billy chose the guard alone over a guard plus a helper
  script. If cutting an edition proves to be a repeated ritual worth automating, that is a
  wish, filed when the third repetition is felt, not designed here.
- **No retroactive editions.** History before `edition-001` gets no tags: the bar cannot be
  re-proven at an old commit without re-running its gates, and pretending otherwise would put
  a claim behind a marker that is supposed to be evidence.
- **Publishing and release naming** stay untouched. An edition is an internal development base,
  not an announcement.
