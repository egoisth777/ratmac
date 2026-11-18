# Independent review before full proof

Ticket [t-149](../../../.arca/ticket/archive/t-149.md), Run `run-073`, requirement WTC-004 (abandon release window).
The independent reviewer built and ran tests only in its own throwaway copies with their own build
folders; it edited no file and ran no git command that changes state in the draft, in the port, or in
either real checkout, and it deleted its scratch folder after each round. In both rounds "caught by" for a
wrong implementation names the public checks `wtcv_010`, `wtcv_011`, and `wtcv_012`
(`test/qa/tests/t136_lock_contention.rs`) and the private lane's six rows, HT-149-01 to HT-149-06, one
test function each.

## Round 1 (2026-11-07)

**Accepted; no blocker or medium finding, two low findings and one surviving wrong implementation**, at
draft commits 48c857a (red) and 6a7ee72 (green), cut on 60997e6. The red commit changes only tests; the
green commit changes only `src/lock.rs` and `src/abandon.rs`.

- **Low 1, a lane helper with no time limit:** the lane's `Fixture::rtm` collected the Engine with a call
  that never times out, against the lane's own statement that every Engine child is collected inside its
  ceiling and killed on expiry. Under V7, a wait that never ends, HT-149-04 and HT-149-05 still hung when
  the reviewer killed them at 240 s. It is not a false pass - HT-149-02, HT-149-03, and HT-149-06 failed
  and `wtcv_012` failed at its 30 s limit - but a lane sweep would report red only at its own 1800 s limit.
- **Low 2, for the landing step:** `.ratmac/lanes.toml` has no `t-149` roster entry, so the roster check
  fails on any tree holding the lane. With the entry added in scratch, the check that no two build targets
  write one file and the scan for sources that read Cargo's build messages pass.
- **One wrong implementation passes every check:** V12 takes the root lock only while waiting on the Run
  lock. HT-149-03's fixture denies every read, so the root read always refuses first and the wait on the
  Run lock is never reached. The green code takes no lock while it settles a presence read, and the
  reconcile locks in `src/abandon.rs` are released before those reads.
- **The test-only fault is absent from a normal build.** An `rtm.exe` built alone for the root package
  holds none of the three fault setting names, and its stand-in function returns nothing. The QA Engine
  and the lane's Engine hold them, because both build with the fault feature.
- **Only an access denial is read again.** The code retries only on error 5, only when the path is not a
  folder, and only on Windows; every other error returns at once. V2, which retries any error code, is
  caught only by `wtcv_012`, which counts record lines and so does not depend on timing.
- **At the bound the answer is a refusal.** The last error is returned; V3, which answers "absent", and
  V4, which answers "present", are caught by `wtcv_012` and HT-149-02 to HT-149-06.
- **The read counts cannot leak between tests.** The fault looks at its selector setting first and
  returns before touching a counter when it is unset; the counters live inside one Engine process and
  every case starts a new one; the selector is only ever passed to a child process. Counting per lock kind
  is intended: the leftover route reads the root lock twice and `wtcv_010` depends on that count; V10, one
  counter for both kinds, is caught.
- **A side effect that follows from the design:** the retire step of `respawn` calls the same abandon
  planning, so it gains the same wait; the successor-ownership suite passes.
- **Red**, on the red commit's source: `wtcv_010` to `wtcv_012` and HT-149-02 to HT-149-06 fail;
  HT-149-01, the regression row, passes by design.
- **Green**, on 6a7ee72: `wtcv_010` to `wtcv_012` and HT-149-01 to HT-149-06 pass. Clippy, for the root
  package alone and for the workspace with `--all-targets`, exited 0 with seven warnings, all in two suites
  this ticket does not touch; `rustfmt --check` was clean on every changed Rust file.
- **Not this ticket:** `wrsv_001_03` in `test/qa/tests/t122_successor_ownership.rs` passed for the
  reviewer on base and on green. The freeze commit 4f78de5 that the `t085` suite needs is missing from
  the draft copy and present in the primary checkout.
- **Older than this ticket:** a root `rtm` built by a whole-workspace build carries every test hook,
  because Cargo merges features across the workspace; the public audit of the shipped binary builds the
  root binary alone.

| Check or row | Valid control before the refused case | Red reason | Wrong implementations caught |
| :--- | :--- | :--- | :--- |
| `wtcv_010` | twins without the fault for the Run and leftover routes; the host's own text for error 5 | the fault's record is missing | V1 V8 V10 V11 V18 |
| `wtcv_011` | an empty reference without the fault and a Run twin | the fault's record is missing | V1 V8 V9 V10 V11 V18 |
| `wtcv_012` | a twin's retirement | exit 0, the Run retired | V1 V2 V3 V4 V5 V7 V8 V18 |
| HT-149-01 | none needed | passes at red by design | none: no wrong implementation changes behavior without the fault |
| HT-149-02 | guidance without the fault | fails | V1 V3 V4 V7 V8 V9 V11 V18 V22 |
| HT-149-03 | present and absent twins | fails | V1 V3 V4 V5 V7 V8 V11 V18 V22 |
| HT-149-04 | a control on the file's presence | fails | V1 V3 V4 V8 V11 V18; under V7 it hung |
| HT-149-05 | a twin reference | fails | V1 V3 V4 V8 V10 V18 V22; under V7 it hung |
| HT-149-06 | a reference without the fault | fails | V1 V3 V4 V7 V8 V18 |

Wrong implementations, each confirmed newly compiled. No description or result is kept in the reviewer's
record for the numbers this list skips.

| Wrong implementation | Result |
| :--- | :--- |
| V1: no retry | caught |
| V2: any error code is read again | caught, by `wtcv_012` only |
| V3: at the bound the answer is "absent" | caught |
| V4: at the bound the answer is "present" | caught |
| V5: a one-second bound | caught |
| V7: the bound starts again on every denial | caught; two lane rows hung instead of failing (low 1) |
| V8: the root and Run labels swapped | caught |
| V9: the guidance route keeps the single read | caught |
| V10: one counter shared by both lock kinds | caught |
| V11: holds the root lock while waiting | caught |
| V12: holds the root lock only while waiting on the Run lock | passes every check |
| V18: a release-window error treated as "present" | caught |
| V22: the selector's lock kind ignored | caught |

Reviewed draft bytes (SHA-256):

- `7e73154263c33cfd57532db5b52139f4e8a810117fedf46d35951bab6804eb17` `src/abandon.rs`
- `5ff2732b773155449901eef39bac4364d86a80af2ba7b8e1929ba0de9ff5b7d4` `src/lock.rs`
- `3601f3ff8f4dd855164e8c08678ad4a5ff0842042847f26400bda6b861028745` `test/qa/tests/t136_lock_contention.rs`
- `841db00ab999dd9e52bec7e1dd6a195fecdd39508a820ce0770bd0c376e9ed4b` `test-hidden/t-149/Cargo.toml`
- `e5c4f3d3cb815d4672ac049b8b27384bdb57c6a63fd2c96aeb8a0223a8c29913` `test-hidden/t-149/Cargo.lock`
- `9cb3a0e12fac74efa576b0c80a67db25baa4c96288d680ca16155f0eb33d875e` `test-hidden/t-149/src/lib.rs`
- `d898cdfa85e8aaa950af80f1253f7a2691e99e9128898ae780f44d64f41f1b21` `test-hidden/t-149/tests/hidden.rs`

## Round 2 (2026-11-15): the port onto the landed tree

t-141 (unique test scratch folders), t-142 (the shared whole-line wait, SVE-005), t-143 (case-scoped test
hooks, SVE-006), t-148 (skill move retry), t-144 (the Cargo-built bootstrap), t-145 (the live Run pin
check), t-146 (the repeated run refusal), and t-147 (the unreadable record refusal) landed after the
draft was cut, so the coordinator rebuilt the draft on the t-147 stamp as two commits. The port is not
independent, and nothing below rests on its own report. In the port `src/abandon.rs`, `src/lock.rs`, and
the lane's `Cargo.toml`, `Cargo.lock`, and `src/lib.rs` are byte-identical to the draft at 6a7ee72, and
the 637 lines the draft's red commit adds to `test/qa/tests/t136_lock_contention.rs` are added in the
same order. Three files are the port's own: `.ratmac/lanes.toml` with `"t-149"` on the roster, for round
1's low 2; `test/qa/tests/t126_shared_private_test_support.rs` with two rows for the lane; and the lane's
`tests/hidden.rs`, which collects every Engine child inside its ceiling, for round 1's low 1, and waits
on its one pause marker through the shared `support::await_marker_line` (SVE-005).

**Accepted, one low finding, no medium or blocker**, at port commits 7bc6f45 (red) and 49e7d97 (green),
cut on 7d4e319. The turn opened on a13890b, which adds only one wishlist sentence to 7d4e319, so the turn
carries the reviewed bytes. The reviewer ran every test with `cargo test --offline --locked -j 4` in its
own clones and build folders, with every `CARGO_*` setting cleared. All five of the coordinator's claims
about the delta held and every fingerprint matched. Both round 1 low findings are fixed. Of 18 wrong
implementations, 14 were caught and 4 were not (low 1).

What the reviewer checked, and on which commits:

1. **The delta.** Against the trunk the port touches five tracked files: the roster (1 line added), t126
   (6 added), t136 (637 added), `src/lock.rs` (117 added), and `src/abandon.rs` (19 added, 12 removed).
   The red commit touches only the three test and roster files, the green commit only the two source
   files. In t136 the 637 added lines equal the draft's red additions in order, in two places (13 lines
   near the top and 624 at the end), and no line of the trunk's file is removed; at red t136 holds 47
   `assert!`, 57 `assert_eq!`, and 5 tests, as the draft's red does. The lane keeps its 6 tests and its
   counts of assertions; its `hidden.rs` differs from the draft only by the `MarkerLine` import, `rtm`
   collecting through `spawn(..).finish()`, and the local `await_marker` replaced by
   `support::await_marker_line(.., MarkerLine::Exact(&holding), MARKER_CEILING) == Ok(holding.clone())`,
   the same check or stricter.
2. **Red, on 7bc6f45.** `wtcv_010` to `wtcv_012` exit 101 with 3 failed; the whole t136 suite has 2
   passed and 3 failed; t126 passes 10 of 10; the lane exits 101, HT-149-01 passing and HT-149-02 to
   HT-149-06 failing. WTCV-010 and WTCV-011 fail because the fault's record is missing, and WTCV-012
   because abandon exits 0 and retires the Run, as the P4 rows say. With the fix but without the retry
   (V1), WTCV-010 and WTCV-011 refuse with `abandon cannot inspect lock ...: Access is denied. (os error
   5)`, and WTCV-012's lasting denial refuses after about 326 ms.
3. **Green, on 49e7d97.** `wtcv_010` to `wtcv_012` pass 3 of 3, t136 5 of 5, t126 10 of 10, and the lane
   6 of 6 at once and again one test at a time. `cargo fmt --check` and `cargo fmt --all --check` for the
   workspace, `rustfmt --check --edition 2021` for the lane's two Rust files, and `python
   tools/check_links.py` exit 0. Clippy prints no warning for the root package with and without the fault
   feature, for the two suites, or for the lane, and five warnings for the workspace, all in older lines
   of t113 and t135.
4. **Neighbours, on 49e7d97.** The public suites t051 (6 tests), t077 (6), t115 (4), t120 (5), and t122
   (4, which covers the retire step of `respawn`) passed, as did the lanes `t-073` (6), `t-136` (6),
   `t-115` (5), `t-122` (6), `t-130` (6), `t-119` (6), and `t-120` (7). `lock_path_present` is used only
   in `src/abandon.rs`; `plan_abandon_in` is reached from the command line, from `src/scheduler.rs` (line
   2600), and in-process from t077 and t115, all covered.
5. **The landed source scans read this lane** (all 92 lanes present). `svev_003`, `svev_015`, and
   `svev_018` in t126 pass on the port as it is. With the lane's row taken out of `MARKER_KEY_CRATES`,
   `svev_015` fails naming the lane; with the old local `await_marker` put back, `svev_015` names its
   lines; with a process-wide `set_var` planted, `svev_018` fails; with a second reader of Cargo's build
   messages, `svev_003` fails; and with the `NON_MARKER_WAITS` row taken out, `svev_015` names
   `await_first_record_line`. After each plant was removed the lane file had its reviewed bytes and the
   three passed again. The reviewer judged the `NON_MARKER_WAITS` row right and the shared wait wrong for
   that function: the record is an append-only log the fault writes, not a pause marker, and the shared
   wait answers a second whole line as other text, while the Engine appends a line every 10 ms and the
   wait polls every 20 ms.
6. **Wrong implementations, rebuilt on the port's green** (49e7d97), each newly compiled. Under V7 the
   lane now fails instead of hanging: HT-149-04 and HT-149-05 each fail after about 30 s with "the Engine
   child outlived 30s and was killed", and the whole lane fails in 32.33 s. V9b, V24, V25, V27, and V28
   are new in this round. The table lists the failing tests.

| Wrong implementation | Failing tests |
| :--- | :--- |
| V1: no retry | `wtcv_010` to `wtcv_012`; HT-149-02, HT-149-03, HT-149-05, HT-149-06 |
| V2: any error code is read again | `wtcv_012` |
| V3: at the bound the answer is "absent" | `wtcv_012`; HT-149-02 to HT-149-06 |
| V4: at the bound the answer is "present" | `wtcv_012`; HT-149-02 to HT-149-06 |
| V5: a one-second bound | `wtcv_012` |
| V7: the bound starts again on every denial | `wtcv_012` (at its 30 s limit); HT-149-02 to HT-149-06 |
| V8: the root and Run labels swapped | `wtcv_010` to `wtcv_012`; HT-149-02 to HT-149-06 |
| V9: the guidance route keeps a plain single read | `wtcv_011`; HT-149-02 |
| V9b: the guidance route reads once under the fault, with no retry | `wtcv_010`, `wtcv_011`; HT-149-02 |
| V10: one counter shared by both lock kinds | `wtcv_010`, `wtcv_011`; HT-149-05 |
| V11: holds the root lock through the whole wait | `wtcv_010`, `wtcv_011`; HT-149-02 to HT-149-04 |
| V12: holds the root lock only while waiting on the Run lock | none (low 1) |
| V18: a release-window error answered as "present" | `wtcv_010` to `wtcv_012`; HT-149-02 to HT-149-06 |
| V22: the fault ignores which lock its selector names | HT-149-02, HT-149-03, HT-149-05 |
| V24: no pause between reads | none (low 1) |
| V25: one second between reads instead of 10 ms | none (low 1) |
| V27: any raw error 5 is a release window, without the check that the path is not a folder | none (low 1) |
| V28: reads the lock before checking the address | HT-149-06 |

7. **The shipped binary.** `cargo build --offline --locked -j 4 -p ratmac` exited 0; its `rtm.exe`
   (6,462,976 bytes, SHA-256 `e37775eea191d1afbf4fd5b21c1246fad982168cddfd6f0289c89edf3ac64797`) holds
   none of `RATMAC_TEST_LOCK_PRESENCE_FAULT`, `RATMAC_TEST_LOCK_PRESENCE_RECORD`, and
   `RATMAC_TEST_LOCK_FAULT`; its only other `RATMAC_TEST_` text is the existing pause names in
   `src/nested.rs` and `src/skill.rs`, which the port does not touch.

The finding:

- **Low 1, four wrong implementations pass every check.** V12, as in round 1: HT-149-03's cell fixes its
  fixture as denials of every read, so the root read never gets past its wait and the wait on the Run
  lock is never reached. New in this round, V24 reads a lock again with no pause, V25 waits one second
  between reads instead of the lock module's 10 ms, and V27 treats any raw error 5 as a release window,
  without `transient_release_window`'s check that the path is not a folder. None breaks a binding cell:
  no cell fixes how often abandon reads a lock again, and none covers error 5 on a folder at a lock path.
  The reviewer asked for the four to be recorded as known limits and for no change to the port.

The reviewer did not run the whole workspace test set, a lane sweep, or the turn tool, and it ran the
Engine in no real checkout. It did not run t085 or any public suite or lane not named above, and nothing
ran on a platform other than Windows: the branches that keep the single read there were compiled but not
run. It deleted its scratch folder and two folders that its passing `t-136` and `t-120` lane runs left in
the system temp folder. Afterwards the form at 49e7d97 and the draft at 6a7ee72 each showed an empty `git
status`; in the primary checkout, where it ran only read-only git commands, the one change was the
coordinator's wishlist edit.

## Known limits carried to the gap record

1. The pace of the wait is not checked. Abandon reading a lock again with no pause between reads (V24),
   or one second apart instead of 10 ms (V25), passes every check, because no binding cell fixes how
   often abandon reads a lock again.
2. Error 5 on a folder at a lock path is not checked. Treating any raw error 5 as a release window,
   without the check that the path is not a folder (V27), passes every check.
3. Taking the root lock only while waiting on the Run lock (V12) passes every check. HT-149-03's cell
   fixes its fixture as denials of every read, so the root read never gets past its wait and the wait on
   the Run lock is never reached.

## Settled by the landing commits

The draft is not landed as it stands. It lands as two commits built on a13890b, the t-147 stamp plus one
wishlist sentence. The red commit ("t-149: prove abandon release window failures") holds the checks in
t136, the roster line, the two t126 rows, and the ticket's "Settled before red" paragraph. Each check
fails there for its stated reason, recorded under `run-073`: today's abandon reads each lock path once
and has no presence fault hook, so `wtcv_010` and `wtcv_011` find no line in the fault's record and
`wtcv_012`'s abandon exits 0 and retires the Run. After the green commit every reviewed file has its
reviewed bytes: `src/abandon.rs` and `src/lock.rs` equal the draft at 6a7ee72 byte for byte, and t136,
the roster, t126, and the lane's four files are the port's that round 2 reviewed.

## Reviewed inputs

Every changed tracked file and every new file of the private lane, at the bytes the full proof runs on.

| Reviewed input | Tracking | SHA-256 |
| :--- | :--- | :--- |
| `.arca/log.md` | tracked, modified | `b87df2372f2ec91cdc54715a64231afe607cc32297b0bba9d244f69c1dc1d822` |
| `.arca/ticket/t-149.md` | tracked, modified | `09a9cd4c1121c0be8e5a86c7ddc8cc8bf2f9b27963194ab0ef3091465c6f2727` |
| `.ratmac/evidence/run-073/WTCV-010.toml` | tracked, new | `3cc8d1815f45c2a81fc66b95c860e180693047756679c6e515bf5e07f4485d64` |
| `.ratmac/evidence/run-073/WTCV-011.toml` | tracked, new | `b57aedd7cd6912d04e353317c03b017fb840c94a6e7f852a5cabac1618f1c9e0` |
| `.ratmac/evidence/run-073/WTCV-012.toml` | tracked, new | `304d049f97d498ab9fd8dbb50539150cac0bba130ebe0f580d3b12a5a5961837` |
| `.ratmac/lanes.toml` | tracked, modified | `a138efbdd4edbee9aa86da7ff561b952d929522359faab23354971271c2c59fe` |
| `src/abandon.rs` | tracked, modified | `7e73154263c33cfd57532db5b52139f4e8a810117fedf46d35951bab6804eb17` |
| `src/lock.rs` | tracked, modified | `5ff2732b773155449901eef39bac4364d86a80af2ba7b8e1929ba0de9ff5b7d4` |
| `test-hidden/t-149/Cargo.lock` | lane, ignored by Git, new | `e5c4f3d3cb815d4672ac049b8b27384bdb57c6a63fd2c96aeb8a0223a8c29913` |
| `test-hidden/t-149/Cargo.toml` | lane, ignored by Git, new | `841db00ab999dd9e52bec7e1dd6a195fecdd39508a820ce0770bd0c376e9ed4b` |
| `test-hidden/t-149/src/lib.rs` | lane, ignored by Git, new | `9cb3a0e12fac74efa576b0c80a67db25baa4c96288d680ca16155f0eb33d875e` |
| `test-hidden/t-149/tests/hidden.rs` | lane, ignored by Git, new | `28d063a98b5a333438b707aa7ea106580bc0e3b696582e14a3e81ee1b94b0190` |
| `test/qa/tests/t126_shared_private_test_support.rs` | tracked, modified | `5c252f1436c93d3a904644b07f394f6b739687a1a38086889e9ed66fa51c0b52` |
| `test/qa/tests/t136_lock_contention.rs` | tracked, modified | `dbc5b6b3613fdb44f4b041d3cb68408643a4d78f921493fd237f0380b90023bc` |
