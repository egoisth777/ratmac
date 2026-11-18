# Independent review before full proof

Ticket [t-145](../../../.arca/ticket/archive/t-145.md), Run `run-070`, requirement LRP-001 (live Run pin check). The
independent reviewer built and ran tests only in its own throwaway clones with their own build folders; it
edited no file and ran no git command that changes state in the draft, in the port, or in either real
checkout, and it deleted its scratch folder after each round. The reports of rounds 1 and 2 are restated
here from the reviewer's own record of them; where that record keeps no figure, none is given. In every
round "passed every check" for a wrong implementation means the doctor suite
(`test/qa/tests/t045_bootstrap_doctor.rs`) and the private lane, not the whole workspace. The lane's six
rows, HT-145-01 to HT-145-06, are one test function each, named in the last table of round 3.

## Round 1 (2026-11-07)

**No blocker; three medium and five low findings**, at draft commits 185eae2 (red), 88af486 (green), and
3b59d32 (the lane's exception row in the scan for sources that read Cargo's build messages), cut on the
t-144 draft at e9f745b. Every suite the coordinator named passed on 3b59d32, and 185eae2 failed for the
reasons the ticket gives: in the doctor suite `lrpv_001` to `lrpv_005`, `bootstrap_resolves_offline`,
`svev_004`, and `svev_005` failed - on the old line `Pin: no pin recorded in .ratmac/evidence.toml`, on
exit 0 where a refusal was required, or, for `lrpv_003`, on the old flat-pin refusal - and all six lane rows
failed on the old pin behavior.

- **Medium 1, a real defect in `tools/rtm.ps1`:** Git prints paths as UTF-8, and the bootstrap decoded them
  in the console's code page. From a linked worktree of a primary checkout whose path has non-ASCII
  letters, under code page 437, the bootstrap missed the primary's live Run: with the pinned Engine changed
  it exited 0 and printed the no-live-Run line. Under code page 65001 it refused correctly.
- **Medium 2, checks weaker than the stated outcome:** HT-145-02 covered only an ordinary linked worktree.
  V28, which does not skip Git's bare-store record, and V20, which drops the fallback for a checkout whose
  Git folder lives elsewhere, passed every check; in separate probes each let a changed pinned Engine
  through with exit 0.
- **Medium 3, checks weaker than the stated outcome:** the refusal belongs on standard error only, and the
  lane's helper rejected only standard output lines that start `Engine:` or `Pin:`. V27, which echoes the
  whole refusal on standard output, passed every check.
- **Low 1:** "never reads the flat file" was shown only with no live Run. V25, which adds a refusal on a
  stale flat pin while a Run is live, passed.
- **Low 2:** the Run-id order is not tested apart from the file system's own order. The fixtures make
  `run-010`, `run-002`, and `run-001`, which this host's file system already lists in the asserted order,
  so V01, without the sort, passed.
- **Low 3:** the no-live-Run line was never checked from a worktree. V17, which names the worktree's own
  `.ratmac` instead of the primary's, passed.
- **Low 4:** no near-miss record showed where "live" ends. V19, a loose status match that ignores letter
  case, passed.
- **Low 5:** nothing shows that the Git settings the bootstrap clears for its own query are put back. V11,
  which does not restore them, passed. A check that starts the script as a child process cannot see that
  child's settings after it exits; the reviewer saw the difference only in a probe that ran the script
  inside one session. It also noted that the `GIT_DIR` leg of `lrpv_004` passes V09, which clears nothing,
  so clearing is proven by HT-145-02 and not by that public leg.

The draft answered with two commits, d7e2969 (red) and f61f1fa (green). The red one added the code page
case, a bare store, and a separate Git folder to HT-145-02, an empty standard output to the lane's refusal
helper, a stale flat pin beside a live Run to `lrpv_002`, a no-live-Run check from the worktree to
`lrpv_004`, and near-miss records to HT-145-03. The green one reads Git's output as UTF-8 through a new
`Invoke-GitQuery`.

## Round 2 (2026-11-07)

**One blocker, formatting only, and one new low finding**, at d7e2969 (red) and f61f1fa (green).

- **Blocker:** `cargo fmt --check` failed on one new assertion in `lrpv_002`, the only formatting
  difference in the workspace.
- **Medium 1, fixed.** V31, without the UTF-8 decoding, and V35, which reads the worktree list the old way,
  fail HT-145-02 at its code page 437 step.
- **Medium 2, fixed but for low 6.** V28 and V20 now fail HT-145-02.
- **Medium 3, fixed.** V27 now fails HT-145-01, HT-145-02, HT-145-03, HT-145-05, and HT-145-06.
- **Low 1, fixed.** V25 fails `lrpv_002`.
- **Low 3, fixed.** V17 fails `lrpv_004`.
- **Low 4, fixed.** V19 fails HT-145-03, which counts a `Planned` record as not live.
- **Low 2 and low 5, open.** V01 and V11 still pass every check.
- **Low 6, new:** the bare-store case had one linked worktree, so "the first checkout Git lists" and "this
  checkout" were the same folder. V32, which uses this checkout's own `.ratmac` when Git's first record is
  the bare store, passed every check; in a probe with two worktrees it exited 0 on a changed pinned Engine
  where the green script refused.
- **Red.** On d7e2969 the doctor suite passed 15 of 15, because its additions guard behavior the first
  green script already had; in the lane only HT-145-02 failed, at the code page 437 step, with exit 0 and
  the no-live-Run line.
- **Green.** With no `CARGO_*` setting and again with `CARGO_TARGET_DIR` set: the doctor suite 15 of 15,
  the stable bootstrap suite 2 of 2, the abandon hints suite 4 of 4, the t-144 lane 9 of 9, and the t-145
  lane 6 of 6, also one test at a time. With no `CARGO_*` setting: the t-102 lane 6 of 6 and, in a clone
  holding every rostered lane, the shared-support suite 7 of 7. Clippy was clean for the doctor suite, the
  shared-support suite, and the lane.

The draft answered by rewriting its last two commits and adding one: 5caea3e (red) is d7e2969 with the
formatting fixed, 2f9d0e7 (green) replays f61f1fa with the same `tools/rtm.ps1` bytes, and fdd85da adds a
second worktree to the bare-store case, a check the script already passed.

## Round 3 (2026-11-07)

**Accepted, two low findings open, no medium or blocker**, at draft commit fdd85da.

- **Blocker, fixed** inside the red commit, so `cargo fmt --check` passes at every commit.
- **Low 6, fixed.** The Run now starts from the second worktree and is expected under the first checkout
  Git lists; V32 fails HT-145-02 at its matching-pin step, printing the no-live-Run line for the second
  worktree.
- **Low 2 and low 5, open.** V01 and V11 pass the doctor suite 15 of 15 and the lane 6 of 6.
- **Red.** On 5caea3e the doctor suite passes 15 of 15 and only HT-145-02 fails in the lane, at the code
  page 437 step, as in round 2.
- **Green**, on fdd85da, with no `CARGO_*` setting and again with `CARGO_TARGET_DIR` set: the doctor suite
  15 of 15, the stable bootstrap suite 2 of 2, the abandon hints suite 4 of 4, the t-144 lane 9 of 9, and
  the t-145 lane 6 of 6, also one test at a time. With no `CARGO_*` setting: the t-102 lane 6 of 6 and, in
  a clone holding every rostered lane, the shared-support suite 7 of 7. `cargo fmt --check` passed for the
  workspace and the lane, `cargo clippy --offline --workspace --all-targets` exited 0 with five warnings in
  two suites this ticket does not touch, Clippy was clean for the lane, and `python tools/check_links.py`
  passed.
- **The lane's exception row** in the scan for sources that read Cargo's build messages stays narrow: with
  the row removed the scan flags exactly one site, a fixed join in the lane.

The reviewer did not run the whole workspace test set in any round.

Wrong implementations over the three rounds, each run against the doctor suite and the lane: round 1 on the
first green script, round 2 on the second, round 3 on fdd85da. A variant is listed under the last round
whose result the reviewer's record keeps. Two still pass every check (low 2 and low 5).

| Wrong implementation | Round | Failing tests |
| :--- | :--- | :--- |
| V01: the Run ids are not sorted | 3 | none (low 2) |
| V02: any changed pin hash refuses, even when the Run pinned another path | 1 | HT-145-05, HT-145-06 |
| V03: only `planned` counts as live, not `executing` | 1 | `lrpv_001`, HT-145-03 |
| V04: every status but `passed` counts as live | 1 | HT-145-03 |
| V05: the Run's status line is not trimmed | 1 | HT-145-03 |
| V06: pinned paths are compared with letter case on Windows | 1 | HT-145-02 |
| V07: the full path is not normalized | 1 | `svev_004`, `svev_005` |
| V08: the invoking checkout's own `.ratmac` is always used | 1 | `lrpv_004`, HT-145-02, HT-145-05 |
| V09: inherited Git settings are not cleared | 1 | HT-145-02 |
| V10: only `GIT_DIR` is cleared | 1 | HT-145-02 |
| V11: the saved Git settings are not put back | 3 | none (low 5) |
| V12: the refusal names only the first changed Run | 1 | HT-145-05 |
| V13: a pin read error escapes | 1 | `lrpv_005`, HT-145-04 |
| V14: the pin read error's text is dropped | 1 | HT-145-04 |
| V15: an unreadable Run record counts as live | 1 | `lrpv_002`, HT-145-03, HT-145-04 |
| V16: the flat pin is read when no Run is live | 1 | `lrpv_003` |
| V17: the no-live-Run line names the local checkout | 2 | `lrpv_004` |
| V18: the refusal drops each Run's evidence file line | 1 | `lrpv_001`, `lrpv_004`, HT-145-01, HT-145-02, HT-145-03, HT-145-05, HT-145-06 |
| V19: a loose status match that ignores letter case | 2 | HT-145-03 |
| V20: no fallback for a separate Git folder | 2 | HT-145-02 |
| V21: `run.toml` is read without guarding read errors | 1 | `lrpv_002`, HT-145-04 |
| V23: `Pin:` lines are printed before changed pins are checked | 1 | `lrpv_001`, `lrpv_002`, `lrpv_004`, `lrpv_005`, HT-145-01 to HT-145-06 |
| V24: only the executable's file name is compared | 1 | `lrpv_002`, HT-145-05, HT-145-06 |
| V25: a stale flat pin refuses while a Run is live | 2 | `lrpv_002` |
| V26: the no-live-Run line is printed only when `runs` is absent | 1 | HT-145-03 |
| V27: the whole refusal is echoed on standard output | 2 | HT-145-01, HT-145-02, HT-145-03, HT-145-05, HT-145-06 |
| V28: Git's bare-store record is not skipped | 2 | HT-145-02 |
| V29: a worktree's bootstrap writes a marker into the shared root | 1 | HT-145-05 |
| V30: a refusal writes a file beside the Run's pin | 1 | `lrpv_001`, HT-145-01, HT-145-05 |
| V31: Git's output is not decoded as UTF-8 | 2 | HT-145-02 |
| V32: a bare first Git record means this checkout's own `.ratmac` | 3 | HT-145-02 |
| V35: the worktree list is read in the console's code page | 2 | HT-145-02 |

The reviewer's record keeps nothing for the numbers V22, V33, and V34.

| Lane row | Test function |
| :--- | :--- |
| HT-145-01 | `ht_145_01_outside_the_pin_lines_the_report_and_the_refusal_keep_their_shape` |
| HT-145-02 | `ht_145_02_a_linked_worktree_reads_the_primary_root_whatever_git_says` |
| HT-145-03 | `ht_145_03_only_live_runs_count_in_run_id_order_and_retiring_clears_a_refusal` |
| HT-145-04 | `ht_145_04_missing_unreadable_or_half_written_records_and_pins_never_stop_the_bootstrap` |
| HT-145-05 | `ht_145_05_one_refusal_names_every_changed_run_once_and_writes_nothing` |
| HT-145-06 | `ht_145_06_a_run_pinning_the_stable_engine_leaves_default_and_nightly_working` |

## Round 4 (2026-11-14): the port onto the landed tree

t-142 (the shared whole-line wait, SVE-005), t-143 (case-scoped test hooks, SVE-006), t-148 (skill move
retry), and t-144 (the Cargo-built bootstrap, on whose draft this draft was cut) landed after the draft was
cut, so the coordinator rebuilt the draft on the t-144 landing as two commits. The port is not independent,
and nothing below rests on its own report. `tools/rtm.ps1` and the four files of the private lane are
byte-identical to the draft at fdd85da. Two files differ from the draft because the trunk moved under them:
`test/qa/tests/t126_shared_private_test_support.rs` is the trunk's file with the lane's exception row, and
`.ratmac/lanes.toml` is the trunk's file with `"t-145"` on the roster. The doctor suite differs from the
draft in one test function, changed in this round for the finding below.

**First pass, one finding**, at port commits a39b51a (red) and 5169c1a (green). The green commit passed
every check the reviewer ran, and it found no regression in the script. On the red commit `lrpv_002` failed
for another reason than its ticket row states: it planted the stale flat pin before its first bootstrap, so
the unchanged script refused with exit 1 on the old flat-pin mismatch, where the row says it exits 0 and
prints `Pin: no pin recorded in .ratmac/evidence.toml`. The coordinator made the fix the reviewer proposed:
`lrpv_002` now bootstraps once with no flat pin and asserts the clean report of the live Run, then plants
the flat pin and bootstraps again; the flat pin case is kept as it was. That is the port's one change to a
reviewed test, 18 lines added and 6 removed in that function.

**Accepted, two low findings open, no medium or blocker**, at port commits 367e4fc (red) and d0d9eb9
(green). The reviewer ran every check in its own clones, with every `CARGO_*` setting cleared. All
fingerprints the coordinator gave matched. What the reviewer checked, and on which commits:

1. **The delta.** Against the trunk the port touches four tracked files: the red commit the roster, the
   doctor suite, and t126; the green commit only `tools/rtm.ps1`. In t126 the only change is the exception
   list growing from 11 to 12 rows with one fixed-join row for `test-hidden/t-145/`; in the roster the only
   change is the `"t-145"` line. The red commit's script is the trunk's, and the green commit's script
   equals the draft's at fdd85da. The new first bootstrap in `lrpv_002` removes none of the stale flat pin
   assertions.

2. **Red, on 367e4fc.** The doctor suite exits 101 with 7 passed and 8 failed, and each of the five checks
   first fails for its row's reason: `lrpv_001` (line 1253) because the changed pinned Engine exits 0 with
   the old no-pin line where exit 1 is expected; `lrpv_002` (line 1334) at its first pin-line assertion,
   with the old line where `matches the pin of run-001` is expected, the exit status assertion before it
   passing; `lrpv_003` (line 1453) because the flat pin refuses with "the resolved Engine is not the pinned
   Engine"; `lrpv_004` (line 1508) at the worktree with no live Run, exit 0 with the old line; and
   `lrpv_005` (line 1607) at the case of a folder in the pin's place, exit 0 with the old line. The other
   three failures are `bootstrap_resolves_offline`, `svev_004`, and `svev_005`, the existing checks the
   ticket adjusts. The lane's red result is from a39b51a and was not repeated on 367e4fc: 0 of 6 pass, every
   row meeting the old flat-file pin report first.

3. **Green.** Rerun on d0d9eb9: the doctor suite 15 of 15; `cargo fmt --check` for the workspace and for the
   lane; Clippy for the doctor suite and t126, and for the lane, with no warning; and `python
   tools/check_links.py`. From 5169c1a, not rerun after the fix, which changes only a test's control: the
   doctor suite 15 of 15, the stable bootstrap suite 2 of 2, the abandon hints suite 4 of 4, t126 10 of 10,
   the t-145 lane 6 of 6 and again one test at a time, the t-144 lane 9 of 9, and the t-102 lane 6 of 6.

4. **The landed source scans read this lane** (on 5169c1a, with all 89 rostered lanes present). `svev_003`,
   `svev_015`, and `svev_018` pass on the port as it is. With a local pause-marker poll planted in the lane
   `svev_015` fails, with a process-wide test-hook setting `svev_018` fails, and with a second reader of
   Cargo's build messages `svev_003` fails, each naming the planted line. A planted redirect refusal fails
   `svev_003` as well, and with the lane's exception row removed `svev_003` names exactly the lane's one
   fixed join. Every plant was restored.

5. **Wrong implementations, rebuilt on the port's green** (5169c1a; V25 again on d0d9eb9). Each replaced
   only the script in a clone and ran the doctor suite, the stable bootstrap suite, the abandon hints suite,
   and the t-145, t-144, and t-102 lanes. The table lists the failing tests; every suite not listed passed.

| Wrong implementation | Failing tests |
| :--- | :--- |
| V01: the Run ids are not sorted | none (low 2) |
| V11: the saved Git settings are not put back | none (low 5) |
| V17: the no-live-Run line names the local checkout | `lrpv_004` |
| V19: a loose status match that ignores letter case | HT-145-03 |
| V20: no fallback for a separate Git folder | HT-145-02 |
| V25: a stale flat pin refuses while a Run is live | `lrpv_002`; on d0d9eb9 its first bootstrap passes and the second fails at line 1352, "a stale flat pin beside a live Run refuses nothing" |
| V27: the whole refusal is echoed on standard output | HT-145-01, HT-145-02, HT-145-03, HT-145-05, HT-145-06 |
| V28: Git's bare-store record is not skipped | HT-145-02 |
| V31: Git's output is not decoded as UTF-8 | HT-145-02, at the code page 437 step |
| V32: a bare first Git record means this checkout's own `.ratmac` | HT-145-02 |
| V35: the worktree list is read in the console's code page | HT-145-02, at the code page 437 step |

The two low findings, both open since round 1:

- **Low 2.** V01, which drops the sort of the Run ids, still passes everything: Windows already lists the
  fixtures' numbered Run folders in the expected order. The reviewer's fix: live Runs with hand-made ids
  whose file system order differs from plain byte order, such as `run-B01` and `run-a01`.

- **Low 5.** V11, which never puts the saved Git settings back, still passes everything: the tests start the
  bootstrap as its own `pwsh -File` process, which hides what it leaves in its environment. The reviewer's
  fix: start the bootstrap from a wrapper in the same PowerShell process and compare the three Git settings
  before and after.

The reviewer did not run the whole workspace test set, Clippy over the whole workspace, the lane sweep, or
the turn tool, and it did not repeat the lanes or the scan plants on the final pair; each result above names
the commit it ran on. One setup attempt fetched a branch name that does not exist and reran the first pair;
the reviewer left it out of the results. It deleted 24 fixture folders its own runs of the stable bootstrap
suite had left in the system temp folder, and its scratch folder.

## Known limits carried to the gap record

1. The Run-id order of the pin lines is not tested apart from the file system's own order. The
   fixtures make `run-010`, `run-002`, and `run-001`, which this host's file system already lists in
   the asserted order, so a script without the sort passes every check here.
2. No check shows that the Git settings the bootstrap clears for its own query are put back. A check
   that starts the script as a child process cannot see that child's settings after it exits; the
   reviewer saw the difference only in a probe that ran the script inside one session.

## Settled by the landing commits

The draft is not landed as it stands. It lands as two commits built on the t-144 landing. The red commit
("t-145: prove live Run pin check failures") holds the checks in the doctor suite, the roster line, the
exception row in t126, and the ticket's "Settled before red" paragraph. Each check fails there for its
stated reason, recorded under `run-070`: `lrpv_001` because the bootstrap exits 0 with `Pin: no pin recorded
in .ratmac/evidence.toml` where the Engine a live Run pinned has changed; `lrpv_002`, `lrpv_004`, and
`lrpv_005` because they get that same line where they expect `Pin: matches the pin of run-001`, the line
naming the primary checkout's `.ratmac` from a linked worktree with no live Run, and the line naming
`run-001`'s unreadable pin; and `lrpv_003` because a flat pin the bootstrap should no longer read refuses
with "the resolved Engine is not the pinned Engine".

After the green commit every reviewed file has its reviewed bytes: `tools/rtm.ps1` and the lane's
`Cargo.toml`, `Cargo.lock`, `src/lib.rs`, and `tests/hidden.rs` equal the draft at fdd85da byte for byte;
the doctor suite is the draft's file with the one test function that round 4 reviewed; t126 and the roster
are the trunk's files with the one exception row and the one roster line that round 4 reviewed.

## Reviewed inputs

Every changed tracked file and every new file of the private lane, at the bytes the full proof runs on.

| Reviewed input | Tracking | SHA-256 |
| :--- | :--- | :--- |
| `.arca/log.md` | tracked, modified | `196ed3cf137ec760d575921457657626f8b2a0d6404645f81459846d09c15eeb` |
| `.arca/ticket/t-145.md` | tracked, modified | `280959c99876c3a1de23b4bbf67007a4ed7f553d8bd8beb9504e29959f1e60c3` |
| `.ratmac/evidence/run-070/LRPV-001.toml` | tracked, new | `e26b8cb7ccec96cb18e7760f1fd5f29ddc761ae3c8785c313fd6b73bd5d223d8` |
| `.ratmac/evidence/run-070/LRPV-002.toml` | tracked, new | `03e5157bdb7456017b786e47afdd5cc7b34b86accd70e498764ad597a0d19d90` |
| `.ratmac/evidence/run-070/LRPV-003.toml` | tracked, new | `022b529d82efc1d824a506c4f5f29ad2c678660179692825ad0574b9e6097633` |
| `.ratmac/evidence/run-070/LRPV-004.toml` | tracked, new | `c7a3474d871fd8049049435f05d2740800081ade8c1c9ec980fe3b8c11dbcab4` |
| `.ratmac/evidence/run-070/LRPV-005.toml` | tracked, new | `015b0cd221b458290ee23b5d5c56b646b2002a896e6b5ecc924a6ec87c96bb8a` |
| `.ratmac/lanes.toml` | tracked, modified | `c5fb539c15598945901db2d73514bb4243cfcd55f6575c7e9f4107327f4691db` |
| `test-hidden/t-145/Cargo.lock` | lane, ignored by Git, new | `665162870b5ca59a2d83cf33a29188748f134142cfbf9d0fc0c0052baa1bd15d` |
| `test-hidden/t-145/Cargo.toml` | lane, ignored by Git, new | `38a3530144fb958149148c1355cfd241be7ab15973952a6e2f1ee1e7955d4c95` |
| `test-hidden/t-145/src/lib.rs` | lane, ignored by Git, new | `8a994c3b06e8d1aa532edf09ebd2462f3872466b281d282cc3acd547d9d080ff` |
| `test-hidden/t-145/tests/hidden.rs` | lane, ignored by Git, new | `41aacd3e782ebf5a274174871429c525b3a584369b83c4d34996204fbf058271` |
| `test/qa/tests/t045_bootstrap_doctor.rs` | tracked, modified | `f27620a4c72e438150773550cb40bbe3d11881bdc5ee952923cafdcbd876d44f` |
| `test/qa/tests/t126_shared_private_test_support.rs` | tracked, modified | `18195b939f8d6bc019392b7e8648919c5ccf26bd7257093aaccdeae077efbe39` |
| `tools/rtm.ps1` | tracked, modified | `9a2288f630fea0ce0912a72eae0fa7f9d3ff527a02f4a4b865f40c96bd453a6a` |
