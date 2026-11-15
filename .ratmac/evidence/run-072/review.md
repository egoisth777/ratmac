# Independent review before full proof

Ticket [t-147](../../../.arca/ticket/t-147.md), Run `run-072`, requirement WTC-002 (unreadable record
refusal). The independent reviewer built and ran tests only in its own throwaway copies with their own build
folders; it edited no file and ran no git command that changes state in the draft, in the port, or in either
real checkout, and it deleted its scratch folder after each round. In both rounds "caught by" for a wrong
implementation names the public checks `wtcv_004` and `wtcv_005`
(`test/qa/tests/t133_selected_issue_completion.rs`), `wtcv_006`
(`test/qa/tests/t135_distributable_plan_build_profile.rs`), the older public check `wcpv_019` in that second
suite, the unit test `profile::tests::the_shipped_pair_verifies`, and the private lane's six rows, HT-147-01
to HT-147-06, one test function each. Round 2 also names `wcpv_020` and `wcpv_021`, two more older checks in
that second suite, and a second profile unit test that its report does not name.

## Round 1 (2026-11-07)

**Accepted; no blocker or medium finding, two low findings**, at draft commits 86bfd34 (red) and d7c9cd2
(green), cut on b8b8054. The red commit only adds checks; the green commit changes only the validator, its
packaged copy, and the manifest; no existing assertion was removed or weakened.

- **Same bytes in both places.** `tools/verify_issue_binding.py` and
  `profiles/plan-build/checkers/verify_issue_binding.py` have one SHA-256, with no carriage return in
  either, the manifest records that digest, and the unit test that verifies the shipped pair passes.
- **Short reason.** Both catches - the item record and the gap record - refuse with the operating
  system's short reason and print the path with forward slashes. Two wrong implementations that print the
  full error text, v01 and v19, were caught.
- **Two choices by the author, both still proving their rows.** HT-147-01 deletes the bound item file,
  the only way to get an absent record after the Engine has bound it; v11, with no existence test, is
  caught there. HT-147-03 checks `.ratmac/log.md`, which is the Engine's history log, so it is the row's
  "history unchanged"; v15, which appends to that log, is caught there.
- **Low 1, for the landing step:** `.ratmac/lanes.toml` has no `t-147` roster entry, so on a tree holding
  the lane the roster check and the lane sweep's own check refuse it as an undeclared lane. With the entry
  added and all 85 lanes present in scratch, the scan for sources that read Cargo's build messages passes.
- **Low 2, one wrong implementation no test can tell apart:** v18 hard-codes the reason
  `Permission denied`. Every fixture a test can build - a hold with no sharing, or permission mode 0 -
  produces exactly that reason. The checks take the reason from a probe of the host rather than a fixed
  text. No fix was asked for.
- **Red**, on 86bfd34: `wtcv_004` to `wtcv_006` and HT-147-02 to HT-147-06 fail on a Python traceback;
  HT-147-01, the regression row, passes by design.
- **Green**, on d7c9cd2, four jobs at a time: the selected-issue suite 5 of 5, the profile suite 4 of 4,
  the lane 6 of 6, the profile unit tests 3 of 3; the neighbouring lanes `t-133` 6 of 6, `t-135` 6 of 6,
  and `t-092` 11 of 11, and the public `t092` suite 2 of 2. `python tools/check_links.py` passed,
  `rustfmt --check` was clean on the changed files, Clippy reported nothing in the new code beside five
  older warnings in two suites, and Clippy was clean for the lane.
- **Not run:** the whole workspace test set; `svev_007` in `test/qa/tests/t139_same_lane_verdict.rs`,
  whose rules the reviewer read instead - the lane calls only `lane_folder` with its own temp folder and
  `support::command`, which that check does not flag. The paths for other platforms were reviewed by
  reading only.

| Check or row | What it proves | Valid control before the refused case |
| :--- | :--- | :--- |
| `wtcv_004` | one exact standard-error line with the short reason, exit 1, empty standard output, no traceback; a probe confirms the file still exists while its open fails; skips are reported | the turn passes before the hold and again after |
| `wtcv_005` | the same, plus a byte snapshot before and after that covers both Run records, the spawn ledger, the item, and the gap record | yes |
| `wtcv_006` | the export succeeds; the exported checker's bytes equal the tool's; for both kinds its exit code and standard-error bytes equal the tool's | both pass with an equal success line, before and after each kind |
| HT-147-01 | absent and folder records keep the "does not exist" row; an unreadable `spec.md` keeps the requirement row; an unreadable unrelated spawn ledger still passes; never a read row or a traceback | a readable record passes |
| HT-147-02 | the first, then the second listed gap record is named; item plus first gap record gives the item line; unlisted records pass | yes |
| HT-147-03 | two runs give identical standard error; `rtm step` refuses with `exit 1;` and the line; the Run record and the log are unchanged; after release one step reaches done | `rtm doctor` is clean and a readable record passes |
| HT-147-04 | a whole-tree snapshot, content and metadata, is unchanged; the record's bytes and read-only flag are unchanged | yes |
| HT-147-05 | exact bytes, one line break, no carriage return or backslash, a reason with no path or error number, 0 bytes of standard output, exit 1 | yes |
| HT-147-06 | a clean repository built only from the export, with the profile's own roots; bytes equal the tool's; both kinds refuse with the exact line; it passes again after | yes |

Wrong implementations, each built in a scratch copy:

| Wrong implementation | Caught by |
| :--- | :--- |
| p01: a stale packaged copy with a matching stale manifest | `wtcv_006`, `wcpv_019`, HT-147-06 |
| p02: a stale manifest | `wtcv_006`, `wcpv_019`, HT-147-06, the shipped-pair unit test |
| p03: a packaged copy with CRLF line ends and a matching digest | `wtcv_006`, `wcpv_019`, HT-147-06 |
| p04: the packaged copy fixed, the tool not | `wtcv_004` to `wtcv_006`, `wcpv_019`, HT-147-02 to HT-147-06 |
| v01 full error text; v04 the two kinds swapped; v05 a backslash path; v06 the refusal on standard output; v07 exit 2; v08 the reason plus a file name; v14 a traceback printed as well; v20 a trailing period | `wtcv_004` to `wtcv_006`, HT-147-02 to HT-147-06 |
| v02: the item record only | `wtcv_005`, `wtcv_006`, HT-147-02, HT-147-04, HT-147-05, HT-147-06 |
| v03 the gap record only; v13 an unreadable item read as empty; v17 a "cannot open" wording | `wtcv_004`, `wtcv_006`, HT-147-02 to HT-147-06 |
| v19: full error text, gap record only | `wtcv_005`, `wtcv_006`, HT-147-02, HT-147-04, HT-147-05, HT-147-06 |
| v09 the tolerant reader refuses; v10 the spawn ledger refuses; v11 no existence test | HT-147-01 |
| v12: unlisted gap records are read too | HT-147-02 |
| v15: appends to `.ratmac/log.md` | `wtcv_005`, HT-147-03, HT-147-04 |
| v16: sets the record read-only | HT-147-04 |
| v18: a hard-coded reason | none (low 2) |

Reviewed draft bytes (SHA-256):

- `68f543970646d61c99e8c1fb82969317d6caec21507814e9601691a883b2e085` `tools/verify_issue_binding.py`
- `68f543970646d61c99e8c1fb82969317d6caec21507814e9601691a883b2e085` `profiles/plan-build/checkers/verify_issue_binding.py`
- `4fc14cf1082b82b22e61f2c0576bf11d5e180e4b6aafefdd2a7e1aa6f9b08e9e` `profiles/plan-build/manifest.toml`
- `12a5e72789eefdfc7cf25a87c38c21b59200a90d96f1fdeae1bab98a08d3f2fd` `test/qa/tests/t133_selected_issue_completion.rs`
- `cc29c5dbb68735717824533a3865683407fd855480a1a2390e9e6bbee6aef5fd` `test/qa/tests/t135_distributable_plan_build_profile.rs`
- `6c434af1b809bb4620a2751d6d3bb1b58e576d1ef4703281ba34718c13e0e93d` `test-hidden/t-147/Cargo.toml`
- `83cb27d942cd53d579386aef6cbee5ab3d707dc5e01f1d9205d0b793f56ce15e` `test-hidden/t-147/Cargo.lock`
- `888500e4a610411ea42f02faf7674e83f9e689db4a68ab031bbb52c262f40d95` `test-hidden/t-147/src/lib.rs`
- `112b43f617eb9cf4f83db60aa5351197e2f39b1f6dbdafec46ee6b28dc9c761c` `test-hidden/t-147/tests/hidden.rs`

## Round 2 (2026-11-14): the port onto the landed tree

t-141 (unique test scratch folders), t-142 (the shared whole-line wait, SVE-005), t-143 (case-scoped test
hooks, SVE-006), t-148 (skill move retry), t-144 (the Cargo-built bootstrap), t-145 (the live Run pin
check), and t-146 (the repeated run refusal) landed after the draft was cut, so the coordinator rebuilt the
draft on the t-146 landing as two commits. The port is not independent, and nothing below rests on its own
report. In the port every file round 1 reviewed is byte-identical to the draft at d7c9cd2: the validator,
its packaged copy, the manifest, the two public suites, and the four files of the private lane. One file is
the port's own: `.ratmac/lanes.toml` is the trunk's file with `"t-147"` on the roster, for round 1's low 1.

**Accepted, three low findings, no medium or blocker**, at port commits a7b06c5 (red) and f59a35b (green),
cut on 6f1ccaa. The reviewer ran every test with `cargo test --offline --locked -j 4` in its own clones and
build folders. All five of the coordinator's claims about the delta held and every fingerprint it gave
matched. Of 32 wrong implementations, 29 were caught and 3 were not (lows 2 and 3).

What the reviewer checked, and on which commits:

1. **The delta.** Against the trunk the port touches six tracked files. The red commit adds 531 lines and
   removes none: the roster (1 line), the selected-issue suite (248), and the profile suite (282). The green
   commit changes the validator and its packaged copy by the same 14 lines added and 2 removed, and one line
   of the manifest. The two suites at the red commit are the draft's exact bytes, so no check was removed or
   weakened. The trunk's validator, packaged copy, and manifest equal the draft base's, and the port's equal
   the draft's green. `"t-147",` sits after `"t-146",` on the roster, t126 is unchanged, and the form's 90
   other lanes match the primary checkout, 1061 files compared and none differing. The packaged copy has no
   carriage return and no byte-order mark, and the manifest records its digest.
2. **Red, on a7b06c5**, run twice with the same result. `wtcv_004` and `wtcv_005` together exit 101 with 2
   failed, `wtcv_006` exits 101, and the lane exits 101 with 1 passed and 5 failed: HT-147-01, the
   regression row, passes by design. Every failure is the no-traceback assertion meeting a Python traceback
   for `PermissionError: [Errno 13] Permission denied`, the reason the ticket's rows give.
3. **Green, on f59a35b**, run twice with the same counts. The selected-issue suite 5 of 5, the profile suite
   4 of 4, t126 10 of 10, the profile unit tests 3 of 3, the lane 6 of 6 and again one test at a time;
   `cargo fmt --all --check` for the workspace and `rustfmt --check` for the lane; Clippy over the two
   suites with four warnings, all in older lines of the profile suite (485, 845, 1334, and 1408), and none
   for the lane; and `python tools/check_links.py`.
4. **Neighbours, on f59a35b.** The public suites t086 (4 tests), t092 (2), t119 (4), and t120 (5) passed, as
   did the lanes `t-092` (11), `t-120` (7), `t-133` (6), and `t-135` (6). Two checks fail there and pass on
   the trunk, which is low 1.
5. **The landed source scans read this lane** (on f59a35b, with all 91 rostered lanes present). `svev_003`,
   `svev_015`, and `svev_018` in t126 pass on the port as it is. With a `RATMAC_TEST_PLANTED` setting
   planted in a lane test `svev_018` fails, with a pause-marker key and a local poll `svev_015` fails, and
   with a second reader of Cargo's build messages `svev_003` fails, each naming the planted line. After the
   plants were taken out the lane's test file had its reviewed bytes and the three passed again.
6. **Wrong implementations, rebuilt on the port's green** (f59a35b), in a scratch clone with all 91 lanes.
   Each ran the selected-issue suite, the profile suite, the profile unit tests, and the lane. In the v rows
   the validator and its packaged copy carry the same wrong bytes and the manifest records their digest, so
   only behavior differs. p05, p06, and v21 to v26 are new in this round. The table lists the failing tests.

| Wrong implementation | Failing tests |
| :--- | :--- |
| p01: the packaged copy and the manifest left at the trunk's bytes and digest | `wtcv_006`, `wcpv_019`, HT-147-06 |
| p02: the manifest keeps the old digest | `wtcv_006`, `wcpv_019` to `wcpv_021`, two profile unit tests, HT-147-06 |
| p03: the packaged copy saved with CRLF line ends, its digest matching | `wtcv_006`, `wcpv_019`, HT-147-06 |
| p04: the validator not fixed | `wtcv_004` to `wtcv_006`, `wcpv_019`, HT-147-02 to HT-147-06 |
| p05: the new digest also written over the `verify_profile.py` row | `wtcv_006`, `wcpv_019` to `wcpv_021`, two profile unit tests, HT-147-06 |
| p06: the packaged copy saved with a byte-order mark, its digest matching | `wtcv_006`, `wcpv_019`, HT-147-06 |
| v01 full error text; v04 the two kinds swapped; v05 a backslash path; v06 the refusal on standard output; v07 exit 2; v08 the reason plus a file name; v14 a traceback printed as well; v20 a trailing period; v22 an absolute path; v24 the reason in lower case; v25 an extra second line | `wtcv_004` to `wtcv_006`, HT-147-02 to HT-147-06 |
| v02 the gap record left unguarded; v19 the gap record's reason as full error text | `wtcv_005`, `wtcv_006`, HT-147-02, HT-147-04, HT-147-05, HT-147-06 |
| v03 the item record left unguarded; v13 an unreadable item read as empty; v17 a "cannot open" wording | `wtcv_004`, `wtcv_006`, HT-147-02 to HT-147-06 |
| v09 the read of `spec.md` refuses; v10 the spawn ledger refuses; v11 no existence test | HT-147-01 |
| v12: every gap record is read, not only the listed ones | `wcpv_021`, HT-147-02 |
| v15: appends to `.ratmac/log.md` | `wtcv_005`, HT-147-03, HT-147-04 |
| v16: sets the record read-only | HT-147-04 |
| v23: writes `refusal.txt` in the fixture | `wtcv_005`, HT-147-04 |
| v18 a hard-coded reason; v21 only the permission error caught; v26 a file written in the system temp folder | none (lows 2 and 3) |

The findings:

- **Low 1, for the close step: the lane sweep's report has no row for t-147 yet.** The roster lists
  `"t-147"` at both port commits, so on the green commit `python tools/sweep_lanes.py check` exits 2 with
  `no verdict row for t-147`, and the roster check `the_roster_is_complete_and_the_check_refuses_strays`
  (RLRV-003) in the t110 suite fails, while both pass on the trunk. Every landed lane stood this way between
  its red commit and its close, t-145 and t-146 included; the sweep at the close writes the row. The
  reviewer asked for no change to the port.

- **Low 2, open since round 1, now with a second wrong implementation.** v18 hard-codes the reason
  `Permission denied`, and v21 catches only the permission error; both pass every check. On Windows every
  unreadable record these checks build, a hold with no sharing, reads `Permission denied`, and permission
  mode 0 on Unix reads the same. The reviewer found no Windows fixture with another reason: a link to an
  app-execution alias is not seen as a file, which gives the "does not exist" row. Linux has one: the
  reviewer made a record a link to `/proc/self/mem` and called the green validator's own `read_record` under
  WSL, which saw a file and answered `cannot read the item record` with the reason `Input/output error`. A
  Unix-only case built on that link would catch both. The reviewer asked for the limit to be recorded and
  for no change to the port.

- **Low 3, new: a write outside the fixture is not seen.** v26 appends to a log file in the system temp
  folder on each refusal and passes every check. That the validator writes nothing is checked inside the
  fixture only, by the byte snapshot in `wtcv_005` and the whole-tree snapshot in HT-147-04, which is what
  those rows say. Giving the validator's process an owned, empty temp folder in HT-147-04 and asserting that
  it stays empty would catch it. The reviewer called that change optional.

The reviewer did not run the whole workspace test set, Clippy over the whole workspace, a lane sweep, or the
turn tool, and it ran the Engine in no real checkout. It did not run the suites that work on the real lanes:
t108's real sweep, `svev_009` in t109, `svev_007` and `svev_008` in t139, and `svev_009` in t140. No suite
ran on Linux: the Unix paths were read only, and the WSL probe called `read_record` alone, not a whole
`check` run. It copied lanes only from the form, never from the primary checkout. It deleted its scratch
folder, which held its clones, build folders, wrong implementations, and logs, and the scratch folder it
made inside WSL. Afterwards the form at f59a35b, the draft at d7c9cd2, and the primary checkout at 6f1ccaa
each showed an empty `git status`.

## Known limits carried to the gap record

1. No check tells the operating system's own reason from the fixed text `Permission denied`, or a catch of
   every read error from a catch of the permission error alone. On Windows every unreadable record a test
   can build, a hold with no sharing, gives exactly that reason, and so does permission mode 0 on Unix, so a
   validator that hard-codes the reason or catches only the permission error passes. The checks take the
   expected reason from a probe of the host, not from a fixed text. On Linux a record that links to
   `/proc/self/mem` reads `Input/output error`; no check uses it.
2. That the validator writes nothing is checked inside the fixture only, by the byte snapshot in `wtcv_005`
   and the whole-tree snapshot in HT-147-04. A validator that on each refusal writes a file outside the
   fixture, in the system temp folder, passes.

## Settled by the landing commits

The draft is not landed as it stands. It lands as two commits built on the t-146 landing. The red commit
("t-147: prove unreadable record refusal failures") holds the checks in the selected-issue suite and the
profile suite, the roster line, and the ticket's "Settled before red" paragraph. Each check fails there for
its stated reason, recorded under `run-072`. `wtcv_004` holds the bound item record unreadable and
`wtcv_005` one of its gap records, and the validator prints a Python traceback where one `cannot read` line
is expected. `wtcv_006` runs the checker exported by `rtm profile plan-build` on the unreadable item record,
and it prints the same traceback. After the green commit every reviewed file has its reviewed bytes: the
validator, its packaged copy, the manifest, the two suites, and the lane's `Cargo.toml`, `Cargo.lock`,
`src/lib.rs`, and `tests/hidden.rs` equal the draft at d7c9cd2 byte for byte, and the roster is the port's
file that round 2 reviewed.

## Reviewed inputs

Every changed tracked file and every new file of the private lane, at the bytes the full proof runs on.

| Reviewed input | Tracking | SHA-256 |
| :--- | :--- | :--- |
| `.arca/log.md` | tracked, modified | `6e95f0faebcd60f12b6d84a757a4e79682723bb4ddf1798e595a3cd8315f238c` |
| `.arca/ticket/t-147.md` | tracked, modified | `b887a155f3788fff376fdb67e6004e182c7b2ef2c6bbf386a4d4af446f5ebd8f` |
| `.ratmac/evidence/run-072/WTCV-004.toml` | tracked, new | `c4eb1fc8dfc26171d9f127dd16b550bddf0d3d88b85c6e265b492a6e1f51259e` |
| `.ratmac/evidence/run-072/WTCV-005.toml` | tracked, new | `e89b7db351bd8c2d5a2501b0c5fe1dfbb823ced7e5905e9f652d6aa7cfd18d2f` |
| `.ratmac/evidence/run-072/WTCV-006.toml` | tracked, new | `0e0d24563b5532a0bbbbdd84c3e331a356f37e5acfa5a861be57f5ae7c7f6196` |
| `.ratmac/lanes.toml` | tracked, modified | `4c66a4882dc084d4eeac032b4e5caed3350b429209770b8bc96c310fb28fabed` |
| `profiles/plan-build/checkers/verify_issue_binding.py` | tracked, modified | `68f543970646d61c99e8c1fb82969317d6caec21507814e9601691a883b2e085` |
| `profiles/plan-build/manifest.toml` | tracked, modified | `4fc14cf1082b82b22e61f2c0576bf11d5e180e4b6aafefdd2a7e1aa6f9b08e9e` |
| `test-hidden/t-147/Cargo.lock` | lane, ignored by Git, new | `83cb27d942cd53d579386aef6cbee5ab3d707dc5e01f1d9205d0b793f56ce15e` |
| `test-hidden/t-147/Cargo.toml` | lane, ignored by Git, new | `6c434af1b809bb4620a2751d6d3bb1b58e576d1ef4703281ba34718c13e0e93d` |
| `test-hidden/t-147/src/lib.rs` | lane, ignored by Git, new | `888500e4a610411ea42f02faf7674e83f9e689db4a68ab031bbb52c262f40d95` |
| `test-hidden/t-147/tests/hidden.rs` | lane, ignored by Git, new | `112b43f617eb9cf4f83db60aa5351197e2f39b1f6dbdafec46ee6b28dc9c761c` |
| `test/qa/tests/t133_selected_issue_completion.rs` | tracked, modified | `12a5e72789eefdfc7cf25a87c38c21b59200a90d96f1fdeae1bab98a08d3f2fd` |
| `test/qa/tests/t135_distributable_plan_build_profile.rs` | tracked, modified | `cc29c5dbb68735717824533a3865683407fd855480a1a2390e9e6bbee6aef5fd` |
| `tools/verify_issue_binding.py` | tracked, modified | `68f543970646d61c99e8c1fb82969317d6caec21507814e9601691a883b2e085` |
