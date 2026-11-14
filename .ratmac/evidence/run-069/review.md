# Independent review before full proof

Ticket [t-144](../../../.arca/ticket/t-144.md), Run `run-069`, requirement SVE-002 (Cargo-built bootstrap
Engine). The independent reviewer built and ran tests only in its own throwaway clones with their own build
folders; it edited no file and ran no git command that changes state in the draft, in the port, or in the
real checkout, and it deleted its scratch folders after each round. In its first round a failed change of
folder made one read-only `cargo metadata --no-deps` probe run inside the real checkout; it wrote nothing
and the status was unchanged. In its second round it corrected its first report, which had said that no
fixture folder was left behind: it had not looked for the `ratmac-t102-*` names, and it could not tell
which of the 90 such folders then in the system temp folder were its own, so it left them. The reports of
rounds 1 and 2 are restated here from the reviewer's own record of them; where that record keeps no figure,
none is given.

## Round 1 (2026-11-07)

**Rejected, one medium finding**, at draft commits c844166 (red) and 6ae8148 (green), cut from the tree
before t-142 landed. The reviewer found `tools/rtm.ps1` correct and the checks short on one property, never
a guessed path after a build: two wrong versions of the script passed both public suites and all six lane
rows.

- **Medium:** after a build, a guessed path was never tested. V02 takes the first existing `release` or
  `debug` binary under the folder Cargo resolves instead of the one Cargo reported; V16 falls back to such a
  binary when Cargo reports none. Both passed every check.
- **Low 1:** a failed `cargo metadata` was not tested. V14, which then falls back to the root's `target`,
  passed everything.
- **Low 2:** with no `cargo` on `PATH` the unguarded `cargo metadata` call ended in a PowerShell error
  instead of a `bootstrap refused:` line.
- **Low 3:** the inside-the-root boundary was not tested. V18, a prefix test without the separator, passed;
  it would reuse a binary from a sibling folder such as `<root>-stable-edition-001/target`.
- **Low 4:** the roster had no `t-144` entry.
- **Low 5:** `svev_006` accepted any refusal line holding `searched:` and `repair:`, the metadata refusal
  included.
- **Low 6:** the earlier bootstrap lane (`t-102`) fails its first row when the host sets `CARGO_TARGET_DIR`.
  It is older than this ticket: on the script before it, five of that lane's six tests fail there.

The reviewer also noted that `svev_003` has no check for an exception row that stops matching anything, so
the scan stays tight only through each row's list of kinds.

The draft answered with two commits. 591417d (red) added three lane tests, planted a binary Cargo did not
report in HT-144-02's reported folder and in HT-144-05's refused case, and made `svev_006` also require
`Cargo reported` in the refusal line. e9f745b (green) made the output-folder question in `tools/rtm.ps1`
catch a missing `cargo` and refuse by name.

## Round 2 (2026-11-07)

**Accepted, four low findings open, no medium or blocker**, at draft commit e9f745b.

- **Medium, fixed.** V02 now fails HT-144-02 and HT-144-05; V16 now fails HT-144-05, where it exits 0 with
  the unreported `debug` binary.
- **Low 1, fixed.** The new `ht_144_05_metadata_failure_never_falls_back_to_the_roots_target` catches V14,
  which reports the root's `target/debug` binary.
- **Low 2, fixed.** The script refuses with "repair: put cargo on PATH", and the new
  `ht_144_05_missing_cargo_refuses_by_name_and_names_the_repair` catches V19 (the first green script), V20
  (a generic repair text), and V21 (a fallback to the root's `target`).
- **Low 3, fixed.** The new `ht_144_03_a_root_prefix_is_not_an_inside_redirect` catches V18, which reuses a
  sibling folder's binary. The test passes on the script before this ticket: it is a guard, not a
  failing-before claim.
- **Low 5, fixed.** `svev_006` also requires `Cargo reported`.
- **Red.** On the first green script the only test failing at 591417d is the missing-cargo one, for want of
  "a `bootstrap refused:` line on standard error". The script before this ticket fails `svev_004`,
  `svev_005`, and `svev_006` in the doctor suite, `svev_005` in the stable bootstrap suite, and lane rows 02
  to 06 with both new HT-144-05 tests; HT-144-01 and the root-prefix test pass on it.
- **Green under five host environments.** With no `CARGO_*` setting, with `CARGO_TARGET_DIR`, with
  `CARGO_BUILD_TARGET_DIR` alone, with both set to absolute paths, and with a relative `CARGO_TARGET_DIR`
  beside `CARGO_BUILD_BUILD_DIR`, the doctor suite passed 11 of 11, the stable bootstrap suite 2 of 2, and
  the lane 9 of 9, and the host's redirect folders stayed empty. Formatting was clean for the workspace and
  the lane, and Clippy for the doctor suite and the lane.
- **Still open, all low:** the roster entry (low 4), which the landing's red commit adds; a stable build
  with no `cargo` on `PATH`, whose build call still ends in a PowerShell error as it did before this ticket;
  the `t-102` lane under a host `CARGO_TARGET_DIR` (low 6); and an older test of the stable bootstrap suite,
  unchanged by this ticket, that leaves two folders in the system temp folder on every run, while this
  ticket's `svev_005` cleans up after itself.

Wrong implementations of round 2, each run against both public suites and the lane on e9f745b. None
slipped; the unchanged script failed nothing.

| Wrong implementation | Failing tests |
| :--- | :--- |
| V02: after a build, a binary found under the resolved folder is preferred to the one Cargo reported | `ht_144_02_only_what_cargo_reported_for_rtm_picks_the_engine`, `ht_144_05_writes_stay_in_the_folders_cargo_resolves_and_a_refusal_names_its_search` |
| V14: a failed `cargo metadata` falls back to the root's `target` | `ht_144_05_metadata_failure_never_falls_back_to_the_roots_target` |
| V16: a binary is guessed when Cargo reports none | `ht_144_05_writes_stay_in_the_folders_cargo_resolves_and_a_refusal_names_its_search` |
| V18: the inside-the-root test has no separator | `ht_144_03_a_root_prefix_is_not_an_inside_redirect` |
| V19: the first green script, with no catch for a missing `cargo` | `ht_144_05_missing_cargo_refuses_by_name_and_names_the_repair` |
| V20: a generic repair text | `ht_144_05_missing_cargo_refuses_by_name_and_names_the_repair` |
| V21: a fallback to the root's `target` | `ht_144_05_missing_cargo_refuses_by_name_and_names_the_repair`, `ht_144_03_outside_the_root_means_build_first_and_stable_keeps_its_own_target`, `ht_144_05_metadata_failure_never_falls_back_to_the_roots_target` |

## Round 3 (2026-11-14): the port onto the landed tree

t-142 (the shared whole-line wait, SVE-005), t-143 (case-scoped test hooks, SVE-006), and t-148 (skill move
retry) landed after the draft was cut, so the coordinator rebuilt the draft on the t-148 landing as two
commits. The port is not independent, and nothing below rests on its own report. It changed no reviewed
byte: `tools/rtm.ps1`, both public bootstrap suites, and the four files of the private lane are
byte-identical to the draft at e9f745b. Two files differ from the draft because the trunk moved under them:
`test/qa/tests/t126_shared_private_test_support.rs` is the trunk's file with the lane's exception row, and
`.ratmac/lanes.toml` is the trunk's file with `"t-144"` on the roster.

**Accepted, three low findings, no medium or blocker**, at port commits 76652bb (red) and ba31712 (green).
The reviewer ran every check in its own clones of the two commits, each with all 88 lanes copied in, with
`CARGO_TARGET_DIR` unset. All fingerprints the coordinator gave matched. What the reviewer checked:

1. **The delta.** Against the trunk the red commit touches only the roster, the two bootstrap suites, and
   t126; the green commit touches only `tools/rtm.ps1`. In t126 the only change is the exception list
   growing from 10 to 11 rows with the `test-hidden/t-144/` row, whose kinds and reason are the reviewed
   ones; in the roster the only change is the `"t-144"` line. The ticket text was the same on the trunk and
   in the draft.
2. **Red.** At 76652bb all four check functions exit 101 with 0 of 1 passing, each for the ticket's reason:
   `svev_004` with "the build succeeded but no Engine binary is at the expected path"; the doctor suite's
   `svev_005` reporting the `target/release` stand-in instead of a fresh build; `svev_006` reporting the
   `target/debug` stand-in with exit 0 where 1 is expected; and the stable bootstrap suite's `svev_005`
   with "the build succeeded but no Engine binary is at the expected path".
3. **Green.** At ba31712 the four functions pass; the doctor suite passes 11 of 11, the stable bootstrap
   suite 2 of 2, t126 10 of 10 with `t-144` on the roster, and the lane 9 of 9. Formatting is clean for the
   workspace and the lane, the link check passes, and Clippy exits 0 with warnings only in t113 and t135,
   which this ticket does not touch.
4. **The two newer rules.** Neither suite nor the lane sets or removes a `RATMAC_TEST_*` setting, calls
   `set_var`, `remove_var`, or `StepFault`, or waits on or reads a pause marker; the lane's "marker" texts
   are fixture engines printing a line, not pause markers. Both scans do read the lane: with a `set_var` of
   a hold-barrier setting planted in a lane test, `svev_018` fails naming it as unlaned; with a
   pause-marker setting and a bare existence wait planted, `svev_015` fails naming the lane as a source that
   "sets a pause-marker key but never calls support::await_marker_line" and as missing from
   `MARKER_KEY_CRATES`. With the plants removed the lane file has its reviewed digest and t126 passes.
5. **What landed under the draft.** In the shared support only an import and the new pause-marker section
   changed since the draft's base; `command`, `scrub`, `inherited_only`, `TempTree`, and `capture` are
   byte-identical, and `inherited_only` already removed `RATMAC_TEST_*` settings at the draft's base. The
   baseline support changed, and none of this ticket's checks use it. `tools/rtm.ps1` on the trunk equals
   the draft's base.
6. **Three wrong implementations, rebuilt on the port's green.** Each is still caught by the same lane row
   as in round 2 (table below), and both public suites pass under all three.

| Wrong implementation | Lane | Caught by |
| :--- | :--- | :--- |
| V02: after a build, the first existing binary under the resolved folder is taken instead of the one Cargo reported | 7 of 9 | `ht_144_02_only_what_cargo_reported_for_rtm_picks_the_engine` and `ht_144_05_writes_stay_in_the_folders_cargo_resolves_and_a_refusal_names_its_search` |
| V14: a failed `cargo metadata` falls back to the root's `target` | 8 of 9 | `ht_144_05_metadata_failure_never_falls_back_to_the_roots_target` |
| V18: the inside-the-root test has no separator, so a sibling folder counts as inside | 8 of 9 | `ht_144_03_a_root_prefix_is_not_an_inside_redirect` |

The three low findings, none in a byte this ticket changes for the worse:

- A stable build with no `cargo` on `PATH` ends in a PowerShell error instead of a `bootstrap refused:`
  line (`tools/rtm.ps1`, the build call in `Invoke-EngineBuild`). The script did the same before this
  ticket; only the output-folder question refuses by name.
- The earlier bootstrap lane (`t-102`) fails its first row when the host itself sets `CARGO_TARGET_DIR`,
  because that lane's script runner does not clear redirects. The lane sweep strips `CARGO_*` settings, so
  the lane passes 6 of 6 there.
- The stable bootstrap suite's `stable_resolves_from_the_invoking_checkout_and_builds_the_tagged_tree`,
  which this ticket does not change, leaves two `ratmac-t102-past-*-stable-edition-001` folders in the
  system temp folder on every run.

The reviewer did not run the whole workspace test set, the lane sweep, or the turn tool, and this round it
did not repeat round 2's redirect matrix, because this round ran with `CARGO_TARGET_DIR` unset; the bytes
that matrix ran on are unchanged. It noted that the tracked lane sweep report has no row for `t-144` until
the turn tool's sweep rewrites it, so the sweep's completeness check refuses on the port until then.

## Known limits carried to the gap record

1. A stable build with no `cargo` on `PATH` ends in a PowerShell error instead of a `bootstrap refused:`
   line. The script did the same before this ticket and no check covers it; only the output-folder
   question refuses by name when `cargo` is missing.
2. `ht_144_03_a_root_prefix_is_not_an_inside_redirect` passes on the script before this ticket. It guards
   against one wrong implementation and is not a failing-before claim.
3. `svev_003`, the scan for sources that read Cargo's build messages, has no check for an exception row
   that stops matching anything, so the lane's row stays tight only through its list of kinds.
4. The five host redirect environments of round 2 ran on the draft and were not repeated on the port,
   whose reviewed bytes are the same; the turn's own runs had `CARGO_TARGET_DIR` unset.
5. Under two of the turn's deliberate faults - a stable build that follows an inherited redirect, and
   reuse that ignores the folder `CARGO_BUILD_TARGET` names - four of the five catching tests fail while
   resolving a folder the faulty bootstrap never created, with the host's file-not-found text instead of
   a row's own message.

## Settled by the landing commits

The draft is not landed as it stands. It lands as two commits built on the t-148 landing. The red commit
("t-144: prove Cargo-built bootstrap failures") holds the checks in both public bootstrap suites, the roster
line, the exception row in t126, and the ticket's "Settled before red" paragraph. Each check fails there for
its stated reason, recorded under `run-069`: `svev_004` because the first run under a moved build output
refuses with "the build succeeded but no Engine binary is at the expected path"; `svev_005` in both suites
under one receipt, in the doctor suite because the bootstrap reports the `target/release` stand-in without
building and in the stable bootstrap suite because the stable build under an inherited `CARGO_TARGET_DIR`
refuses with the same message; and `svev_006` because, where Cargo reports no Engine, the bootstrap reports
the `target/debug` stand-in and exits 0 instead of 1.

After the green commit every reviewed file has its reviewed bytes: `tools/rtm.ps1`, both public bootstrap
suites, and the lane's `Cargo.toml`, `Cargo.lock`, `src/lib.rs`, and `tests/hidden.rs` equal the draft at
e9f745b byte for byte; t126 and the roster are the trunk's files with the one exception row and the one
roster line that round 3 reviewed.

## Reviewed inputs

Every changed tracked file and every new file of the private lane, at the bytes the full proof runs on.

| Reviewed input | Tracking | SHA-256 |
| :--- | :--- | :--- |
| `.arca/log.md` | tracked, modified | `ca0648ee6958875007c9a0b788ccb119ae240cb114a3368301374be532e63e37` |
| `.arca/ticket/t-144.md` | tracked, modified | `51ee640d1487d5398c4248e59841a94371e0c10add6383266ee6675c8686aebb` |
| `.ratmac/evidence/run-069/SVEV-004.toml` | tracked, new | `06542e2f753696e502b26fd250d7dbce0a0bea4d343bd47c4b13021e0b8e5db3` |
| `.ratmac/evidence/run-069/SVEV-005.toml` | tracked, new | `faad23d18b5fec7fd8e5a4e5c657b30cb18ca80e84655379c92c521c225fe854` |
| `.ratmac/evidence/run-069/SVEV-006.toml` | tracked, new | `2d80a648d1525a91d05f2569211ca4d7ce6b53f5a802cd3051dc973de93d9eb2` |
| `.ratmac/lanes.toml` | tracked, modified | `71e31b49da16be6bacf863380e9ca69e884eee3194058f03d44013168f503394` |
| `test-hidden/t-144/Cargo.lock` | lane, ignored by Git, new | `7686fea1cde90fd56c5f44f28766549dfadb4e0c3e4d1ae1292c5cfd1ddc7a58` |
| `test-hidden/t-144/Cargo.toml` | lane, ignored by Git, new | `6733bfc843c1b4be24b4be205c4b98744a87b73362298f11c0c96f68a32203f0` |
| `test-hidden/t-144/src/lib.rs` | lane, ignored by Git, new | `d9a44b375a69f0a0c6253b9ad0b27adbae51bf54307e0742ed90541f305c29f9` |
| `test-hidden/t-144/tests/hidden.rs` | lane, ignored by Git, new | `590cc7dc9a6b07b3d881b77e6a6e7334e135d9e50f579caba7a6ee365316e2df` |
| `test/qa/tests/t045_bootstrap_doctor.rs` | tracked, modified | `f6b2ed84de784d6114a8fe3000def5b14ee0ceae5943badc44ae09e884b348e2` |
| `test/qa/tests/t102_stable_bootstrap.rs` | tracked, modified | `1854a04b20950b6e7e180e1d830b7a4b45b26e591087cf01775deef939195090` |
| `test/qa/tests/t126_shared_private_test_support.rs` | tracked, modified | `d68207b1aac1b08c964627394cb6231e2356815f9a76991adfd9af7b6e78395c` |
| `tools/rtm.ps1` | tracked, modified | `9ecdb9cdbb6b683befcdd16e2c0084dc9e18c760ffb3fe47513a8347ad59eb7d` |
