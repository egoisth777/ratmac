# Issue design

## Proposed mechanics

1. **The sweep is a shop-lane runner, not an Engine feature.** One command -
   a mode of `tools/rtm.ps1` or a sibling script; P1 chooses the owner -
   enumerates `test-hidden/t-*/` in id order, runs each crate's tests from
   the primary checkout (one `cargo test` per crate; the crates are
   standalone, `ratmac = { path = "../.." }`, so a crate always tests the
   tree that holds it - `test-hidden/t-105/Cargo.toml:7-14`), and writes one
   report artifact: per crate, `pass | expired | red` with failing lane ids,
   plus totals. The Engine keeps knowing nothing about lanes; the sweep is
   contributor tooling in the shape of `tools/check_links.py`.

2. **Expiry is a marker a crate carries, written by an explicit act.** A
   marker names the crate, the last-good edition, the date, and the reason;
   the sweep reads it before running and reports `expired` with that edition.
   Marking is never automatic: the sweep proposes, a human or an authorized
   repair lands the marker. The 2026-08-10 classes are the seed use:
   `t-078` and `t-079` take markers naming their last-good edition; the
   repaired `t-071`..`t-083` need none; the pre-split `t-058`..`t-070`
   become the first visible decision this marker exists to hold.

3. **The close wiring is a runbook edit.** The report lands under a declared
   root, and the `close` State's Exit Guard gains one `command_exit`-class
   guard over a checker that exits `0` only when every crate reads `pass` or
   `expired` - the same shape as the edition guard's
   `git describe --exact-match --match edition-*` (`schema.md:518-528`).
   `EDN-002`'s guard is untouched; this one sits beside it. The wiring is
   permissive: a runbook that declines to read the verdict stays legal, and
   the ask is what the verdict must mean when one does.

This file is incoming evidence. Integrated mechanics remain authoritative
only in the accepted forward authority.

## Open decisions for P1

- **Marker home.** Inside the crate - it travels with the copy-back
  discipline and dies with a fresh clone, but so does the crate itself
  (`schema.md:125`) - or a tracked table under `.arca/`, reviewable but
  naming crates a fresh clone does not carry. The wish's own words, "an
  explicit expiry marker a lane carries", read as in-crate; a tracked
  summary may still cite it.
- **Schedule.** The wish leaves it to a human. Proposed: the sweep runs on
  demand, and the close guard is what makes it load-bearing; a standing
  cadence adds nothing until someone asks for it.
- **Expired lanes: run or skip.** Skipping is cheaper and truer to
  "expired"; running keeps the marker honest, because a lane that quietly
  passes again can be un-marked. Proposed: skip by default, with an explicit
  verify mode for the honest check.
- **Sweep ownership and report shape.** `tools/rtm.ps1` mode, a new script,
  or a `ratmac-qa` bin target (t-086's target rules give a qa bin a home);
  the report's path and format must be declared wherever a guard reads it.
