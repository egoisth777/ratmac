# Landed lanes stay runnable, or carry their expiry

```yaml
issue-id: "i-037-landed-lanes-stay-runnable"
provenance: "Wishlist, `Keep a landed ticket's private lanes runnable, or mark them expired` - Billy's shop, filed 2026-08-10; re-evidenced at filing: the 2026-08-21 sprint landed t-102..t-105 into a `test-hidden/` that now holds t-058..t-105 with no runner"
status: "integrated"
```

## Summary

A hidden lane proves its ticket's landing, then nothing ever runs it again.
The runbook touches the lanes exactly twice per ticket - inside the ticket
worktree at P5, and once from `main` as the post-merge confirmation
(`.arca/schema.md:265`, `.arca/schema.md:133`) - and after that no rule,
tool, or workspace member names `test-hidden/`: the folder is gitignored
(`.gitignore:24`), sits outside the Cargo workspace (the root `Cargo.toml`
declares `members = ["test/qa"]` only, and each hidden crate carries its own
empty workspace table to stay out - `test-hidden/t-105/Cargo.toml:7-14`), and
nothing under `tools/` references it. The wish's 2026-08-10 evidence is what
that silence costs: a hand-taken sweep found `t-058`..`t-070` refusing
(pre-split `.arca/` fixtures), `t-071`..`t-083` broken by renames the shop
had authorized later and repaired the same day, and `t-078`/`t-079` refusing
by their own design against frozen source hashes later tickets legitimately
moved (`.arca/wishlist.md:26`). The 2026-08-21 sprint then added four crates
without changing any of this: `t-102`..`t-105` each declare a six-lane
manifest, landed green, and closed the sprint
(`.arca/log.md:432-437`) - their last run anywhere is that landing-time
confirmation. Forty-eight crates now sit in the folder, and a red lane today
means nothing: it is unread history, indistinguishable from a regression.

This issue proposes the wish's either/or as one durable state, in three asks:
a sweep that runs every landed lane and reports `pass` or `expired` per crate
(`LNR-001`); an explicit expiry marker naming the edition a lane last passed
at, so rot is a visible, dated fact rather than silence (`LNR-002`); and the
close stage's permission to read that verdict, so no sprint rests on lanes
nobody ran (`LNR-003`).

## Routes

| Need | File |
| :--- | :--- |
| Terms | [Ubiquitous language](ubi-lang.md) |
| Requirements | [Specification](spec.md) |
| Proposed mechanics | [Design](design.md) |
| Verification and integration traces | [Test plan](test-plan.md) |

## History

- 2026-08-24: filed from the wishlist by the DESIGNER; dispositions are the
  author's proposal, P1 confirms or revises at integration.
- 2026-08-24: P1 integrated all three asks as accepted goal rows
  LNR-001..LNR-003; the marker-home, schedule, and sweep-owner choices are
  recorded at ADR-0020 in the goal design.
