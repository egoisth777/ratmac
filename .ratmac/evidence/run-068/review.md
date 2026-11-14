# Independent review before full proof

Ticket [t-148](../../../.arca/ticket/t-148.md), Run `run-068`, requirement WTC-003 (skill move retry).
The independent reviewer built and ran tests only in its own throwaway clone with its own build folder; it
edited no file and ran no git command that changes state in the draft, in the port, or in the real checkout,
and it left no scratch folder behind. In its third round its first two launches used a shell that cannot see
this host's drive paths; nothing ran, and it ran them again with the right shell.

## Round 1 (2026-11-07)

The independent reviewer read the first draft, cut from the tree before t-142 landed, and built wrong
variants against it. Its round-1 report was not kept; what follows is what the lane's header and its round-2
report restate. Three cases were added, each marked "Added after the first review" in
`test-hidden/t-148/tests/hidden.rs` and each written for one wrong implementation:

1. HT-148-02 gained "another failure after a denial": once a first move was denied, a move that fails with
   another error must still refuse at once. It is written for an implementation that keeps retrying every
   error after a first denial (variant V18 below).
2. HT-148-04 gained case (c): a refusal at the bound removes the partial folder when the hold does not block
   the removal. It is written for an implementation whose refusal leaves the partial folder (V08).
3. HT-148-05 (b) gained an upper limit: the refusal comes sooner than 7 s after the release, not only no
   sooner than 5 s. It is written for an implementation with four times the bound (V09).

Round 1 also left a note about carriage-return line endings in the draft's working copy. Round 2 found the
working copy matching the commit for every changed file, and the committed bytes without carriage returns.

## Round 2 (2026-11-07)

**Accepted, two low findings, no medium or blocker**, at draft commit c999a9e.

- The only file changed since round 1 was the lane's `tests/hidden.rs` (87 lines added, 4 removed, the
  removed ones reworded comments); no assertion or helper was removed, and `src/skill.rs` and the public
  suite had the bytes of round 1.
- Rebuilt by the reviewer from the green bytes, each wrong variant failed exactly one lane row and the other
  five passed: V08 at HT-148-04 (c), V09 at HT-148-05 (b) with a refusal 20.026 s after the release, V18 at
  the new HT-148-02 case with a refusal after 5.0175 s.
- At the draft's red commit WTCV-007, WTCV-008, WTCV-009 and HT-148-02 to HT-148-06 failed because `rtm skill`
  never published the pause before the first attempt and exited 0; HT-148-01 passed, as a regression row
  should.
- **Low:** the roster had no `t-148` entry. The landing's red commit adds it.
- **Low:** HT-148-04 (c) assumes that a hold sharing read, write, and delete does not block removing the
  folder. That is true on this host; where it is not, the case would fail on correct code.
- The reviewer also noted that HT-148-05 (b) measures from the move pause's release, which leaves about 1.7 s
  of headroom inside its 7 s limit, and that the new HT-148-02 case and HT-148-04 (c) go further than the
  ticket's oracle cells as written while staying inside each row's stated rationale.

## Round 3 (2026-11-14): the port onto the landed tree

t-142 (the shared whole-line wait, SVE-005) and t-143 (case-scoped test hooks, SVE-006) landed after the
draft was cut, so a helper of the coordinator ported the draft onto the t-143 landing. The port is not
independent, and nothing below rests on its own report. It changed three things: the public suite and the
lane wait through the shared `support::await_marker_line` with the exact boundary line instead of a wait of
their own; the suite's `start` and both waits spell their marker paths from the pause tree in place, because
the marker inventory (`svev_015`) flagged the shared-wait calls as waiting on a path no marker setting was
seen to name; and `MARKER_KEY_CRATES` and the roster gained the suite and the lane.

**Accepted, no new finding above information level, no medium or blocker**, at port commits 3f2a2ab (red)
and c83fdcc (green); the low finding about HT-148-04 (c) is carried from round 2. All fingerprints the
coordinator gave matched. What the reviewer checked:

1. **The delta, row by row.** No row body, assertion, hold, or timing constant changed. The four marker and
   release paths and the eight settings the child receives are the same as the draft's, the selectors and
   the pause limit are unchanged, and the lane's `start` is untouched. `Armed::marker` is read only after the
   command has exited; no wait goes through it. One behavior differs: marker text with a line break that is
   not its last byte used to fail at once and now counts as not yet published until the command exits or the
   limit passes. No row stages marker text, and the Engine writes the boundary line in one write.
2. **The three wrong implementations, rebuilt on the port's green.** Each fails the same one row and passes
   everything else (table below).
3. **Four wrong waits.** A wait that returns as soon as the marker file exists, in the suite (W1) or in the
   lane (W2), passes every row; only `svev_015` catches it, naming the source that "sets a pause-marker key
   but never calls support::await_marker_line". Without the two new `MARKER_KEY_CRATES` rows (W4), `svev_015`
   fails naming both as missing. A prefix wait in place of the exact one (W3) slips every check: the scan
   cannot tell the two apart, the Engine always writes the exact line, and SVE-005 allows a prefix wait.
4. **Case-scoped hooks (SVE-006).** The suite and the lane have no `set_var`, `remove_var`, `StepFault`, or
   hook-lane wrapper; every setting goes through `.env` on a `support::command` child. `svev_018` passes on
   green and, with a `set_var` planted in `wtcv_009`, fails naming it as unlaned.
5. **Red.** At 3f2a2ab `wtcv_007`, `wtcv_008`, and `wtcv_009` fail with "rtm skill never published the whole
   line" for `before-first-attempt` after the command exited 0 with today's success line, and the lane's rows
   02 to 06 fail the same way while 01 passes. The t126 suite passes 10 of 10 there, so the red commit fails
   only for the stated reason.
6. **The respelled paths are needed.** With each respelling undone, `svev_015` fails with "shared wait on a
   path no marker setting is seen to name": the closure back in the suite's `start` (5 call sites), the
   suite's wait going through `self.marker`, and the lane's. An exception row would be wrong, because these
   markers are named by `_MARKER` settings and an exception would hide the site from the scan.
7. **Nothing else weakened.** The Engine source under `src/` is byte-identical between the draft's base and
   the port's base, so carrying `src/skill.rs` over unchanged is valid, and no reference to the removed
   local wait remains.
8. **Green counts:** the operator skill suite 5 of 5 twice, the t-148 lane 6 of 6 twice, t126 10 of 10.
   Formatting is clean for the workspace and the lane. Clippy warns only in t113 and t135, which t-148 does
   not touch.

Wrong implementations, all built on c83fdcc. F: the check fails, so it catches the variant; pass: the
variant slips.

| Wrong implementation | WTCV-007 to 009 | HT-01 | HT-02 | HT-03 | HT-04 | HT-05 | HT-06 |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| V08: a refusal keeps the partial folder | pass | pass | pass | pass | F `:1163` | pass | pass |
| V09: the bound is four times the lock module's wait bound | pass | pass | pass | pass | pass | F `:1229` | pass |
| V18: other errors are retried once a denial has happened | pass | pass | F `:976` | pass | pass | pass | pass |

Wrong waits, on c83fdcc:

| Wrong wait | `svev_015` | Operator skill suite | t-148 lane |
| :--- | :--- | :--- | :--- |
| W1: the suite returns as soon as the marker file exists | F | pass | not run |
| W2: the lane returns as soon as the marker file exists | F | not run | pass |
| W3: a prefix wait instead of the exact line | pass | pass | pass |
| W4: the two new `MARKER_KEY_CRATES` rows dropped | F | not run | not run |

The reviewer did not run the whole workspace test set, the full lane sweep, or the link check, and it noted
that the tracked lane sweep report has no row for `t-148` until the turn tool's sweep rewrites it at landing.

## Known limits carried to the gap record

1. A prefix wait in place of the exact whole-line wait is caught by no t-148 check (W3); SVE-005 allows it.
2. HT-148-04 (c) assumes that a hold sharing read, write, and delete does not block removing the partial
   folder. That holds on this host; where it does not, the case fails on correct code.
3. The sharing-violation attack in HT-148-02 runs only where a probe shows the host produces raw code 32 for
   a held folder; elsewhere the row prints why and skips that attack.
4. HT-148-05 (b) measures from the move pause's release, shortly before the bound starts, so a slow host has
   less than 2 s of room inside its 7 s upper limit; the reviewer saw about 1.7 s.
5. A wait that only checks that the marker file exists is caught by the marker inventory alone, not by any
   row of the suite or the lane (W1, W2).

## Settled by the landing commits

The draft is not landed as it stands. It lands as two commits built on the t-143 landing. The red commit
("t-148: prove skill move retry failures") holds all three checks, the roster line, the two
`MARKER_KEY_CRATES` rows in t126, and the ticket's "Settled before red" paragraph. Each check fails there
for its stated reason, recorded under `run-068`: `wtcv_007`, `wtcv_008`, and `wtcv_009` each wait for the
pause before the first attempt and fail with "rtm skill never published the whole line" for
`before-first-attempt`, because the code has no pause and the command exits 0 with today's success line.
After the green commit every reviewed file has its reviewed bytes: `src/skill.rs` and the lane's
`Cargo.toml`, `Cargo.lock`, and `src/lib.rs` equal the draft at c999a9e byte for byte; the operator skill
suite and the lane's `tests/hidden.rs` differ from it only by the port that round 3 reviewed; and the
roster line and the two t126 rows, which the draft did not have, are the port's.

## Reviewed inputs

Every changed tracked file and every new file of the private lane, at the bytes the full proof runs on.

| Reviewed input | Tracking | SHA-256 |
| :--- | :--- | :--- |
| `.arca/log.md` | tracked, modified | `d53991c2de9c6c2b8eacb8f8dee9eda7a4b450176d09f3eba6b940d21816aa9c` |
| `.arca/ticket/t-148.md` | tracked, modified | `a5255b6be1bca37130b80ac0c6827060eb6cdf393b8188b4356153e467d31d53` |
| `.ratmac/evidence/run-068/WTCV-007.toml` | tracked, new | `9ccb2eea1e7ffcd680ef082844fe019ae1b4fc74a33a491c9cbc85ef46fe9476` |
| `.ratmac/evidence/run-068/WTCV-008.toml` | tracked, new | `5fd84e5fb16c92fa84fb7ce3e016e52bc99aa80bccaf1c162e7c94b584005e35` |
| `.ratmac/evidence/run-068/WTCV-009.toml` | tracked, new | `0c07ba55374ac78874850d152ba393ad19ed011f2d80b810626958c254e53df7` |
| `.ratmac/lanes.toml` | tracked, modified | `b049ca822b99d663f12674a250d526bf0364f5afcaaafdd360407e32e8cb1baf` |
| `src/skill.rs` | tracked, modified | `0f49a507de8dc1434f5896855ac6b3d7df0bf235e4c1aeec21859f148783843a` |
| `test-hidden/t-148/Cargo.lock` | lane, ignored by Git, new | `597db47de25d631f4c0c53169f30fe0698c127da1b05096cb173eb595d7d3e1f` |
| `test-hidden/t-148/Cargo.toml` | lane, ignored by Git, new | `bea238403663a2260c60a36516db15cd2ff4b4fc81110b4e9a5cc11ea8e30898` |
| `test-hidden/t-148/src/lib.rs` | lane, ignored by Git, new | `b951e4089244003a98e7f4bad842dbbeb9d599bfb2c26e4e725330989648d828` |
| `test-hidden/t-148/tests/hidden.rs` | lane, ignored by Git, new | `a11b43db6ae6fcdddcc0e700bc6a67686a105619fa6826c8360074d02913cadc` |
| `test/qa/tests/t104_operator_skill.rs` | tracked, modified | `81d5a0d2712f52c363a03f16b1c29fcf2c79798c09af973843bbd02865f7b8ab` |
| `test/qa/tests/t126_shared_private_test_support.rs` | tracked, modified | `99a22dc2b8c1514eed2516d5768123dfe54152d44873b12a3d619c5662a4388a` |
