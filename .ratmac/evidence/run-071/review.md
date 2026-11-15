# Independent review before full proof

Ticket [t-146](../../../.arca/ticket/t-146.md), Run `run-071`, requirement WTC-001 (repeated run refusal).
The independent reviewer built and ran tests in its own throwaway copies with their own build folders, apart
from one slip in round 2, stated there, in which three runs used the primary checkout as their working
folder and changed nothing in it; it edited no file and ran no git command that changes state in the draft,
in the port, or in either real checkout, it ran the Engine only in its own scratch fixture, and it deleted
its scratch folder after each round. In both rounds "caught by" for a wrong implementation names the public
checks `wtcv_001` and `wtcv_002` (`test/qa/tests/t117_descriptive_run_roster.rs`), `wtcv_003`
(`test/qa/tests/t120_residue_refusal_precedence.rs`), and the private lane's six rows, HT-146-01 to
HT-146-06, one test function each. Round 2 also names `wrsv_005_04`, an older check in that roster suite,
and `wrsv_004_01` to `wrsv_004_09`, the public recovery suite (`test/qa/tests/t132_rule_fix_recovery.rs`).

## Round 1 (2026-11-07)

**Accepted; no blocker or medium finding, two low findings**, at draft commits 86fe3cd (red) and 12b039c
(green), cut on 4b4034e.

- **The change.** A helper, `repeated_run`, and the repeat check placed first in the `--run` arm of the
  five handlers: `hold`, `resume`, `recover`, `spawn`, and `respawn`. `status`, `step`, `intent`,
  `abandon`, and the shared parser `addressed_run_with_roots` are untouched; the diff has hunks only in
  those five functions and the new helper.
- **Precedence.** The check runs inside the argument loop, before the repeated option's value is read:
  after the residue check and before every lookup, read, or write.
- **Exit code and text.** Exit 2 and `<command>: --run given twice; address exactly one run; `, then the
  same roster line and rows that command's missing-value refusal prints, with no `next:` line.
- **Seen on a green binary** in the reviewer's fixture: `hold` and `spawn` with two `--run` values print
  that refusal byte for byte; `hold --run run-001 --run` gets the repeat refusal; `hold --run --run` gets
  the missing-value refusal, because the first `--run` already lacks its value.
- **Low 1, an exception the lane's row does not state:** HT-146-02 says pure argument errors are ordered
  by position. `hold` checks `--review` and `--approve` before the loop, so on green
  `hold --run run-001 --run run-002 --review bad` exits 1 with
  `hold: --review "bad" is not shaped <label>:<signature file>`, and `--approve` behaves the same way.
  The ticket and the design both place the check inside the existing loop, so an option checked before the
  loop cannot follow the order by position; `step` already behaves this way and must not change. The lane
  proves the before-and-after order for all five commands with options the loop parses.
- **Low 2, a control the public check does not hold:** `wtcv_001` says "otherwise valid arguments", but
  the arguments it gives `recover` are only well-formed - the decision file is a placeholder and the Run
  has no enrolled trust - so at red `recover` refused at its trust check instead of acting. The ticket's
  cell asks for no control per command. V14, in which `recover` refuses every `--run`, passes `wtcv_001`
  to `wtcv_003` and is caught by HT-146-01, HT-146-02, HT-146-03, and nine failures in the public
  recovery suite (`test/qa/tests/t132_rule_fix_recovery.rs`, `wrsv_004_01` to `wrsv_004_09`).
- **Red**, on 86fe3cd: `wtcv_001` to `wtcv_003` fail; HT-146-01, the regression row, passes by design;
  HT-146-02 to HT-146-06 fail.
- **Green**, on 12b039c: the two public suites pass, 11 tests, and the lane passes 6 of 6.
  `cargo fmt --check` was clean for the workspace and the lane, and
  `cargo clippy --offline --workspace --all-targets` exited 0 with warnings only in two suites this ticket
  does not touch.
- **Not this ticket:** `wrsv_001_03` in `test/qa/tests/t122_successor_ownership.rs` failed twice on the
  base commit 4b4034e, the second time alone on one thread. That suite never passes `--run` twice.

| Check or row | Valid control before the refused case | Red reason |
| :--- | :--- | :--- |
| `wtcv_001` | a twin acts for `hold`, `resume`, `spawn`, and `respawn`; the `status`, `step`, `intent`, and `abandon` controls run first; none for `recover` (low 2) | 10 misses: four commands act on `run-002`, `recover` refuses on trust |
| `wtcv_002` | a twin `spawn` mints `run-003` and a ledger entry | `spawned run run-003 ... from run run-002` |
| `wtcv_003` | each command's missing-value refusal, bytes and reads, captured before its repeat; residue rows compare one address with the repeat byte for byte | 12 of 12 rows miss |
| HT-146-01 | all five act, `recover` on the governed fixture | passes at red by design |
| HT-146-02 | all five act on a twin, governed `recover` included | 17 misses |
| HT-146-03 | yes | 19 misses |
| HT-146-04 | yes | 7 misses |
| HT-146-05 | every control runs before the refused rows | 45 misses |
| HT-146-06 | a linked worktree's missing-value refusal lists the primary's Runs | 10 misses |

Wrong implementations, each built in a scratch copy. No description or result for a V13 is kept in the
reviewer's record.

| Wrong implementation | Caught by |
| :--- | :--- |
| V1: the check runs after the value is read | HT-146-02 |
| V2: the refusal gains a `next:` line | `wtcv_001`, `wtcv_003`, HT-146-02 to HT-146-06 |
| V3: exit 1 instead of 2 | `wtcv_001` to `wtcv_003`, HT-146-02 to HT-146-06 |
| V4: `recover` has no check | `wtcv_001` to `wtcv_003`, HT-146-02, HT-146-03, HT-146-05, HT-146-06 |
| V5: the check runs after the loop | HT-146-02 |
| V6: a scan before the loop | HT-146-02 |
| V7: refuses only when the two values differ | `wtcv_001`, `wtcv_002`, HT-146-03, HT-146-05, HT-146-06 |
| V8: one extra record read | `wtcv_003`, HT-146-02 to HT-146-06 |
| V9: `abandon` uses the short form | `wtcv_001` |
| V10: the shared parser loses its `next:` line | `wtcv_001` |
| V11: the repeat is refused before the residue check | `wtcv_003`, HT-146-02 |
| V12: the refusal writes to `log.md` | `wtcv_002`, `wtcv_003`, HT-146-02 to HT-146-06 |
| V14: `recover` refuses every `--run` | HT-146-01, HT-146-02, HT-146-03, and the public recovery suite (9 failures) |

Reviewed draft bytes (SHA-256):

- `87e403b03fa432b9778c41cf306d4f13d2e00b14c15eeebec6f6945a81dcec02` `src/cli.rs`
- `b7aec481fc319375d3f009d5499c32311c81558e1a978c3c97b02ddb2ef2d87a` `test/qa/tests/t117_descriptive_run_roster.rs`
- `da8cf42652b04714adba1e84d80e144fcb5889c2ce8266084b913d14047ee816` `test/qa/tests/t120_residue_refusal_precedence.rs`
- `56f350af33cc6be055048d99b760da9c48b28aaccaea8f14af735bb6d78f4f3b` `test-hidden/t-146/Cargo.toml`
- `b570f283f3eaeed87f4d1b7f01e8352cf9e089287e079b5a8813d078fa8f412a` `test-hidden/t-146/Cargo.lock`
- `6515eca39fe7a858b53fffade5b6c5fdbf66e6dc008fd57b52f31d159da5ee11` `test-hidden/t-146/src/lib.rs`
- `420d6ee16a7835f7f7bf3dbeeb655a3928ade978f5eecff421cc40726b8cca23` `test-hidden/t-146/tests/hidden.rs`

## Round 2 (2026-11-14): the port onto the landed tree

t-141 (unique test scratch folders), t-142 (the shared whole-line wait, SVE-005), t-143 (case-scoped test
hooks, SVE-006), t-148 (skill move retry), t-144 (the Cargo-built bootstrap), and t-145 (the live Run pin
check) landed after the draft was cut, so the coordinator rebuilt the draft on the t-145 landing as two
commits. The port is not independent, and nothing below rests on its own report. `src/cli.rs` and the four
files of the private lane are byte-identical to the draft at 12b039c. Three files differ from the draft
because the trunk moved under them or because of round 1: `test/qa/tests/t120_residue_refusal_precedence.rs`
is the trunk's file with the draft's 292 added lines, `.ratmac/lanes.toml` is the trunk's file with
`"t-146"` on the roster, and `test/qa/tests/t117_descriptive_run_roster.rs` is the draft's file with one
comment in `wtcv_001` grown from one line to four, for round 1's low 2.

**Accepted, two low findings, no medium or blocker**, at port commits 24b7d6e (red) and 22c91ab (green), cut
on afa78db. The reviewer ran every test with `cargo test --offline --locked -j 4`, its own build folder, and
every `CARGO_*` setting cleared. All eight fingerprints the coordinator gave matched, and all 18 wrong
implementations were caught.

**One slip, the reviewer's own.** Its first runner for the wrong implementations used two PowerShell
variable names that differ only in letter case, which PowerShell reads as one name, so the path of its
scratch tree was overwritten and, between 14:19 and 14:22, three test runs used the primary checkout as
their working folder. Each built the trunk's own code into the reviewer's scratch build folder and ran the
trunk's own roster and residue suites, whose fixtures live in the system temp folder. The reviewer stopped
the runner, threw those results away, and ran every wrong implementation again behind a guard that refuses
any folder outside its scratch. Nothing changed in the primary checkout: no file or folder there, `.git` and
`target` included, was newer than the reviewer's scratch folder, `git status` was empty, and HEAD was still
afa78db. The coordinator found that checkout clean at afa78db when it opened the turn.

What the reviewer checked, and on which commits:

1. **The delta.** Against the trunk the port touches four tracked files, 688 lines added and none removed:
   the red commit the roster (1 line), the roster suite (365), and the residue suite (292); the green commit
   only `src/cli.rs` (30). The red commit's `src/cli.rs` is the trunk's, which equals the draft base's, and
   the green commit's equals the draft's at 12b039c. The residue suite gains the same 292 lines as in the
   draft and loses none of the trunk's; the trunk's file differs from the draft base only in t-143's
   `library_lane` change. The trunk's roster suite equals the draft base's, and the port's differs from the
   draft's only in that one comment; no code changed. The roster gains the one line `"t-146"`, and its 90
   entries match the 90 lane folders in the same order. t126 is unchanged. No check was removed or weakened,
   and the parts of the shared test support these checks use are unchanged on the trunk.

2. **Red, on 24b7d6e.** Each public check alone exits 101 for its row's reason: `wtcv_001` with 10 misses,
   each of the five commands going on with the last value; `wtcv_002` with `spawned run run-003 (spawn rev)
   from run run-002`; and `wtcv_003` with 12 of 12 rows showing the later check's refusal or the action, its
   residue rows passing. t126 passes 10 of 10. The lane exits 101 with 1 passed and 5 failed: HT-146-01
   passes by design, and HT-146-02 to HT-146-06 fail with 17, 19, 7, 45, and 10 misses, the draft's counts.
   Every miss is a missing repeat refusal or its effect; no control failed.

3. **Green, on 22c91ab,** with all 90 rostered lanes present. The roster suite 6 of 6, the residue suite 5
   of 5, and t126 10 of 10; each public check alone; the lane 6 of 6 and again one test at a time; `cargo
   fmt --check` for the workspace and `rustfmt --check` for the lane; Clippy with no warning for the
   `ratmac` crate, for the two public suites, and for the lane; and `python tools/check_links.py`.

4. **Neighbours, on 22c91ab.** One run of 17 neighbouring public suites passed, 76 tests: t050, t065, t066,
   t068, t074, t075, t077, t087, t103, t116, t119, t122, t125, t129, t130, t132, and t134. `wrsv_001_03`,
   which failed twice on the draft base in round 1, passed. A scan of all 330 Rust files under `test/qa` and
   of the 90 lanes for a call that gives one of the five commands `--run` twice found only this ticket's
   checks and the t-074 lane's `spawn worker --run <parent> --workspace --run`, where `--workspace` refuses
   first: on the green binary, `--workspace needs a directory path`, exit 1.

5. **The landed source scans read this lane** (on 22c91ab, with all 90 rostered lanes present). t126 passes
   10 of 10 on the port as it is. With a process-wide setting of `RATMAC_TEST_OPERATION_LOG` planted in the
   lane `svev_018` fails, with a local poll on a pause-marker key `svev_015` fails, and with a second reader
   of Cargo's build messages `svev_003` fails, each naming the planted line. Every plant was restored;
   afterwards the lane's test file had its reviewed bytes and t126 passed again.

6. **Wrong implementations, rebuilt on the port's green** (22c91ab). Each replaced only `src/cli.rs` in a
   clone and ran the roster suite, the residue suite, and the lane; V4 and V14 also ran the public recovery
   suite. V15 to V19 are new in this round. The table lists the failing tests; every test not listed passed,
   and each failure message names that implementation's own fault. As in round 1 there is no V13.

| Wrong implementation | Failing tests |
| :--- | :--- |
| V1: the check runs after the value is read | HT-146-02 |
| V2: the refusal gains a `next:` line | `wtcv_001`, `wtcv_003`, HT-146-02 to HT-146-06 |
| V3: exit 1 instead of 2 | `wtcv_001` to `wtcv_003`, HT-146-02 to HT-146-06 |
| V4: `recover` has no check | `wtcv_001` to `wtcv_003`, HT-146-02, HT-146-03, HT-146-05, HT-146-06; the public recovery suite still passes 9 of 9 |
| V5: the check runs after the loop | HT-146-02 |
| V6: a scan before the loop | HT-146-02 |
| V7: refuses only when the two values differ | `wtcv_001`, `wtcv_002`, HT-146-03, HT-146-05, HT-146-06 |
| V8: one extra Run Record read | `wtcv_003`, HT-146-02 to HT-146-06 |
| V9: `abandon` uses the short form | `wtcv_001` |
| V10: the shared parser loses its `next:` line | `wtcv_001`, `wrsv_005_04` |
| V11: the repeat is refused before the residue check | `wtcv_003`, HT-146-02 |
| V12: the refusal writes to `log.md` | `wtcv_002`, `wtcv_003`, HT-146-02 to HT-146-06 |
| V14: `recover` refuses every `--run` | HT-146-01, HT-146-02, HT-146-03, and `wrsv_004_01` to `wrsv_004_09` |
| V15: the roster is read from the worktree, not the shared Engine root | HT-146-06 |
| V16: the first `--run` wins | `wtcv_001` to `wtcv_003`, HT-146-02 to HT-146-06 |
| V17: `respawn`'s refusal names `spawn` | `wtcv_001`, `wtcv_003`, HT-146-02, HT-146-03, HT-146-05, HT-146-06 |
| V18: the refusal writes a lock file | `wtcv_002`, `wtcv_003`, HT-146-02 to HT-146-06 |
| V19: `abandon` checks the repeat before the value | HT-146-02 |

The findings:

- **Low 1, open since round 1, now with a second case.** On the green binary, in a scratch fixture whose
  bytes were unchanged afterwards: `hold --run run-001 --run run-002 --review bad` exits 1 with the
  `--review "bad" is not shaped` refusal, and `--approve` with no file exits 1 with `--approve needs the
  approver's signature file`. A well-shaped `--review a:b` in the same place, or a `--blocker` after the
  second `--run`, gets the repeat refusal, exit 2, and `hold --run run-001 --blocker --run run-002` gets the
  `--blocker` refusal, exit 1. `step` behaves the same way with a malformed `--review` and is outside this
  ticket. The second case: `recover` only collects a stray word during its argument loop and refuses it
  after the loop, so `recover --run run-001 stray --run run-002` gets the repeat refusal, exit 2, where
  `hold`, `resume`, `spawn`, and `respawn` refuse the stray word first, exit 1. HT-146-02's own wording does
  not cover either case. The reviewer asked for no code change: both orders come from how those commands
  already read their arguments, which this ticket does not own.

- **Low 2, new: a build folder in the form's lane.** The coordinator's form held a Cargo build folder of 528
  MB inside the lane, where its note to the reviewer said there was none. The source scans skip build
  folders, so no check is affected. The coordinator copied only the lane's four files into the turn's
  worktree.

- **Round 1's low 2 is answered.** The port's comment in `wtcv_001` is accurate: the roster suite gives
  `recover` only a placeholder decision file and enrolls no trust, HT-146-01 and HT-146-02 each run a
  governed `recover` that acts on `run-001`, and V14 fails all nine of `wrsv_004_01` to `wrsv_004_09`.

The reviewer did not run the whole workspace test set, Clippy over the whole workspace, the lane sweep, or
the turn tool, and it ran the Engine in no real checkout. Public suites that give each of the five commands
one `--run` only, and the other private lanes, were covered by the scan and the t-074 probe instead of a
run. It deleted its scratch folder, which held all its clones, build folders, wrong implementations, plants,
logs, and the fixture.

## Known limits carried to the gap record

1. `hold` reads `--review` and `--approve` before its argument loop, so `hold --run run-001 --run run-002
   --review bad` exits 1 with the `--review` shape refusal instead of the repeat refusal; `step` already
   behaves this way. The order by position holds for the options the loop reads, which HT-146-02 proves for
   all five commands.
2. `recover` only collects a stray word during its argument loop and refuses it after the loop, so `recover
   --run run-001 stray --run run-002` exits 2 with the repeat refusal, where `hold`, `resume`, `spawn`, and
   `respawn` refuse the stray word first with exit 1. `recover` read its arguments this way before this
   ticket.

## Settled by the landing commits

The draft is not landed as it stands. It lands as two commits built on the t-145 landing. The red commit
("t-146: prove repeated run refusal failures") holds the checks in the roster suite and the residue suite,
the roster line, and the ticket's "Settled before red" paragraph. Each check fails there for its stated
reason, recorded under `run-071`. `wtcv_001` gives each of `hold`, `resume`, `recover`, `spawn`, and
`respawn` two `--run` values, first different and then the same, and none of the ten prints the refusal:
each goes on with the last value. `wtcv_002` runs the requirement's example, and it spawns `run-003` under
`run-002`. `wtcv_003` pairs the repeat with a defect that refuses later, and all 12 rows miss: the later
check's refusal or the action appears instead.

After the green commit every reviewed file has its reviewed bytes: `src/cli.rs` and the lane's `Cargo.toml`,
`Cargo.lock`, `src/lib.rs`, and `tests/hidden.rs` equal the draft at 12b039c byte for byte; the roster
suite, the residue suite, and the roster are the port's files that round 2 reviewed.

## Reviewed inputs

Every changed tracked file and every new file of the private lane, at the bytes the full proof runs on.

| Reviewed input | Tracking | SHA-256 |
| :--- | :--- | :--- |
| `.arca/log.md` | tracked, modified | `404fc6d24cdede62167e89a4045b9fdfde5bab05da2263d1598918c8cbaf55f0` |
| `.arca/ticket/t-146.md` | tracked, modified | `99a8acd55742ba4477442c29db36a26e3bf14c3e4fe07aa16a3061ea98124e25` |
| `.ratmac/evidence/run-071/WTCV-001.toml` | tracked, new | `a7f3ecce228f5d0213609ae89bc95a4998a8b5e61b1df340aec00e89040efb63` |
| `.ratmac/evidence/run-071/WTCV-002.toml` | tracked, new | `60806dbd6b819c754eda4e6feefc424c7d466b9226f6ba68a6595e6b15635b35` |
| `.ratmac/evidence/run-071/WTCV-003.toml` | tracked, new | `4686e0174b9e615903a8328874e07c75aeeef97f5200cb3e33c559f07834c0ec` |
| `.ratmac/lanes.toml` | tracked, modified | `cfe8ee46dfe0d98309cdca25e7fcdcf435ccb1c00fd2fc2d816518c80c68e13e` |
| `src/cli.rs` | tracked, modified | `87e403b03fa432b9778c41cf306d4f13d2e00b14c15eeebec6f6945a81dcec02` |
| `test-hidden/t-146/Cargo.lock` | lane, ignored by Git, new | `b570f283f3eaeed87f4d1b7f01e8352cf9e089287e079b5a8813d078fa8f412a` |
| `test-hidden/t-146/Cargo.toml` | lane, ignored by Git, new | `56f350af33cc6be055048d99b760da9c48b28aaccaea8f14af735bb6d78f4f3b` |
| `test-hidden/t-146/src/lib.rs` | lane, ignored by Git, new | `6515eca39fe7a858b53fffade5b6c5fdbf66e6dc008fd57b52f31d159da5ee11` |
| `test-hidden/t-146/tests/hidden.rs` | lane, ignored by Git, new | `420d6ee16a7835f7f7bf3dbeeb655a3928ade978f5eecff421cc40726b8cca23` |
| `test/qa/tests/t117_descriptive_run_roster.rs` | tracked, modified | `1dec93419aeff2b2929160a585382c62476189a1e9322799d54fdd7902880502` |
| `test/qa/tests/t120_residue_refusal_precedence.rs` | tracked, modified | `b0eaff25d83311324308e6663169acbf310f2ade316ec7f434e4c7f6934688c0` |
