# Independent review before full proof

Ticket [t-142](../../../.arca/ticket/t-142.md), Run `run-066`, requirement SVE-005 (whole pause marker line).
Every reviewer built and ran tests only in its own throwaway clone with its own build folder; none edited a
file or ran a git command that changes state in the draft, the lane draft or either real checkout. Three
slips, each reported to the coordinator: the shared-wait reviewer's background shell
once ran under PowerShell and wrote a 69-byte cargo error file, `E:\lane-build.txt`, which it deleted; the
private-lane reviewer wrote one stray dependency-list file to the shell's root folder, which it deleted, and in
rounds 1 and 2 ran its scratch test program directly, so the program started in the real checkout, where its
checks then found nothing of its own written; and before the coordinator's no-`cd` rule, the shared-wait and
migration reviewers ran read-only commands with `cd` and `;` joins, and the private-lane reviewer joined steps
with `;`.

## Round 1 (2026-11-07)

Three reviewers split the work, all at draft commit 423eba0 ("green: t-142 rustfmt t126"): one took the shared
wait `await_marker_line` in `test/qa/src/support.rs` and the public checks `svev_013` and `svev_015` (t126) and
`svev_014` (t125); one the move of every wait site in the 9 public suites and the private lanes onto the shared
wait; and one the new private lane `test-hidden/t-142/`.

- **Shared wait and public checks: rejected on one medium finding.** `await_marker_line` reads before it checks
  its limit, so it reads at least once even at a zero limit; a missing file, an empty read or text without its
  closing newline stays not yet published; a line with an inner newline or invalid UTF-8 is other text; Exact
  means equality and Prefix means `starts_with`, both on one whole line, and other text returns at once. It
  uses only the standard library and only reads. No publisher changed: `git diff` from the base commit 59d341f
  over `src`, `tools`, `test/qa/src/bin` and the Cargo files is 0 lines. `svev_013` proves its cell with four
  fixed states, a 200 ms limit and a folder check that the wait only reads; its red reason is right (the red
  commit 7830dc5, "wip: t-142 core red parts", has a stub that panics "the shared whole-line wait does not
  exist yet"). `svev_014` covers the turn tool's `after-validate` and the six Engine pause points that write
  their marker in place (hold barrier, lock hold, policy, nested, endpoint, recover), each after a control
  with nothing planted. All 17 `svev_015` exception rows are genuinely not pause-marker waits. On the base tree
  (59d341f plus the t-141 lanes) the scanner finds 138 sites in 33 files, covering all 13 named lanes, the
  t-130, t-131 and t-132 lanes and all 9 public suites. t126 passed 9 of 9, t125 4 of 4, and the t-142 lane
  6 of 6, twice. Five findings:
  1. Medium: the `svev_015` scanner (`marker_inventory`) could be blinded. Sample sources came back with 0
     sites for a marker read through a same-crate helper, a lane-local `wait_line` reading through such a
     helper, a `for` loop polling `exists()`, an alias made after the setting, a helper that sets the key on
     its own local, `OpenOptions…open`, `Path::exists(&m)`, `symlink_metadata`, and keys built with `static`
     or `format!`. Fixed in red commit 11fc18a ("red: t-142 review checks", one sample source per gap) and
     green commit 1c58adf ("green: t-142 review checks", the scanner fix). One deliberate narrowing: a
     field-held marker passed to a same-crate one-shot read helper outside a wait loop is not followed,
     because following fields gave 2 false sites in t-132; it is declared as a known limit.
  2. Low: a lane the scanner skips went unnoticed unless it owned an exception row (skipping t-125 with a
     planted `while !marker.exists()` loop passed). Fixed: a pinned list, `MARKER_KEY_CRATES`, of the 26
     crates that set a marker key, which must match exactly.
  3. Low: a blank exception reason passed. Fixed: every reason must be non-blank.
  4. Low: nothing documented or tested that the wait reads once even at a zero limit, which the post-exit
     checks at `t086:405`, `t127:214` and `t130:3026` rely on. Fixed: two doc lines in `support.rs` (commit
     6b039df, "green: t-142 zero-limit doc") and zero-limit rows in `svev_013`.
  5. Low: no commit holds `svev_014` red (7830dc5 has only the red `await_marker` body; the test first
     appears in the green commit e6e1620, "green: t-142 shared whole-line wait"), and the committed
     `svev_015` red (67f012c, "red: t-142 svev_015") fails on 5 false positives (t-132 lines 2357 and 2788,
     t-136 lines 1412, 1452 and 1538), not on the ticket's red reason. The reviewer rebuilt both red runs. Not
     changed: no commit reorder; carried below as a known limit.

  In this round three wrong waits passed both public checks and were caught only by the private lane: no
  read at a zero limit (HT-142-06), a prefix match where Exact is required, and retrying on other text (both
  HT-142-02). The ticket assigns the last two to HT-142-02; the first also fails `svev_013` from round 2.
- **Migration: accepted, no blocker or medium finding.** Every site in the slice uses the shared wait, and
  each Exact or Prefix choice matches what the code under `src/` writes. Limits are unchanged, no assertion
  was removed, no `#[test]` was added, removed or renamed, every child-exit early stop is kept, and no local
  pause-marker wait loop is left. The lanes that set a `RATMAC_TEST_*_MARKER` key are exactly the 16 lanes
  changed in commit 4f586aa (the private lane migration), and the public suites that set one are exactly the
  9 that schema.md line 1222 names. Six wrong variants (W1, W2, W3, W5, W6, W7: existence loops put back in
  t050, t128 and t-080, a first-read compare in t134, `w_marker.is_file()` put back at t-136 line 941, and a
  t-073 hold marker sent through the exempt `wait_for_files`) were each caught by `svev_015` at the exact site.
  The public suites passed (t050 7/7, t086 4/4, t122 4/4, t127 3/3, t128 4/4, t130 9/9, t134 4/4, t136 2/2),
  and lanes t-073, t-080, t-081, t-087, t-122 and t-124 passed 6 of 6 on both the draft and the real t-141
  copies; t-125 passed 6 of 6 on the draft, and its t-141 run was cancelled. Four low findings:
  1. In the t-128, t-129, t-130, t-132 and t-134 lanes, failure messages were reworded, not extended, so they
     no longer show the marker path and the expected text. Nothing reads these strings. Fixed in round 2.
  2. In t-124 and t-131 `paused_kill` (t-125 follows the same pattern), a whole wrong line now goes to
     `Err(wait)` and reports "boundary marker never appeared (…other text…)" instead of failing the old
     assert_eq message; the `Ok(line) => assert_eq!` arm can never fail. It still fails, only the wording
     changed. Fixed in round 2.
  3. The t-127 lane's `ht_127_04` now polls `while !ran_marker.is_file()` locally. The file is written by
     `git config`, not named by a `RATMAC_TEST_*_MARKER` setting, so this is allowed. Not changed.
  4. Contract text, not the draft: the "Replace …" list at schema.md line 1222 leaves out the t-130 and t-132
     lanes and the already-whole waits that were also moved (t-129, t-130 and t-132 `wait_for_file`, the t-134
     lane's `wait_for_file`, the t-136 lane's `wait_marker` and `wait_hold_marker`). Schema.md line 1220 ("no
     lane keeps its own wait") and the ticket's boundary list cover them. Not changed.
- **Private lane: rejected on three medium findings.** In each, a row proved a weaker property than its oracle
  cell, and the reviewer built a wrong `await_marker_line` that passed all six rows. Against the red stub at
  7830dc5 all six rows fail with "the shared whole-line wait does not exist yet (SVE-005 red stub)". The lane
  passed 6 of 6 at default threads and at `--test-threads 1`. Five findings:
  1. Medium: HT-142-05's writer used Rust's default file sharing, so nothing modelled `WriteAllText`, which
     lets others only read while it writes. A wait that opens the marker for writing passed all six rows.
  2. Medium: every stopped state in HT-142-03 and HT-142-04 was planted before the wait started, so a wait
     that reports its first unfinished read instead of its last passed all six rows.
  3. Medium: HT-142-01 and HT-142-03 only compared the returned text, so a wait that returns the expected
     exact line as soon as the marker first changes (the run-053 failure) passed all six rows.
  4. Low: HT-142-05 compared only before-and-after snapshots, so a wait that writes and deletes a sibling file
     on every poll passed all six rows.
  5. Low: `publisher_round` hard-coded "HT-142-06" in its messages, so a failure in row 02 was reported as
     row 06.

  All five were fixed in one commit on a separate lane draft, 64b2cf4 ("green: t-142 lane review checks"):
  4000 rewrites in HT-142-05 with the odd ones sharing like `WriteAllText`, live writers stopping at every
  step in rows 03 and 04, a check in row 03 that the wait did not return before the whole write, a folder
  watcher and modified-time check in row 05, and `HT-142-{label}` messages. A note, no action: the lane's
  `Cargo.lock` pins newer crates than the root lock, as the t-137 and t-138 lanes do.

## Round 2 (2026-11-07)

The round-1 fixes entered the draft as 11fc18a, 1c58adf and 6b039df (shared wait and checks), 1ac36e6 (a
cherry-pick of the lane fix 64b2cf4) and 790091f (a wording-only commit in 8 lanes' `tests/hidden.rs`).

- **Shared wait and public checks re-review: rejected on one medium finding.** At 790091f round-1 findings 2,
  3 and 4 are confirmed fixed: a blank reason, a dropped row, a skipped t-125 and an unpinned t-136 each now
  fail `svev_015`, and a wait with no read at a zero limit now fails `svev_013` ("at limit 0ns"). t126 passed
  9 of 9 and t125 4 of 4; clippy on t126 shows 0 warnings and `cargo fmt --check` is clean. Three findings:
  1. Medium: `svev_015` reported 0 sites on green but could not see 13 real pause-marker waits. The reviewer
     put a raw `while !<arg>.exists()` poll in front of each of the 103 `await_marker_line` calls; 10 wait on
     markers HT-142-02 to 05 make themselves and are out of scope, and of the other 93, 80 were flagged and
     13 missed: 7 in t-122 (key set through the lane wrappers `pause_envs` and `lock_hold_envs`, path waited
     on as `held.marker`), 4 in t-128 (path rebuilt by `hold_paths(label).0`), 1 in t-132 (key relayed as
     `RATMAC_PRIVATE_T132_MARKER` and re-set with `set_var`) and 1 in t-142 (key built by
     `format!("{selector}_MARKER")`). Fixed by a coverage floor in red commit 558e26d and green commit 8d2c250
     (both "coverage floor"): the first argument of every `await_marker_line` call must be recognised as a
     marker, directly or through a wrapper parameter at each caller, or be covered by a named
     `UNKEYED_SHARED_WAITS` row; the four missed shapes are recognised, each with a control row.
  2. Low: sample sources still missed a marker held in a struct field and read through a crate-local helper,
     a helper that returns a struct holding the marker, a `const` key in another module, and a `concat!` key.
     Partly fixed with the coverage floor; what remains is listed in round 3 and carried as a known limit.
  3. Low: `svev_014` has never been committed red; rebuilt, it fails for the right reason. Carried as a known
     limit.
- **Private lane re-review at 64b2cf4: rejected on one medium finding.** All five round-1 findings are fixed:
  opening the marker for writing fails row 05 with 61 to 137 failed writes per run, the stale first read is
  caught by rows 03 and 04, the early exact return by rows 03 and 04, the sibling writer by row 05's watcher,
  and messages say `HT-142-{label}`. The lane passed 6 of 6 at default threads (21.0 s) and at
  `--test-threads 1` (45.1 s), and all six rows fail on the red stub. Two findings:
  1. Medium: HT-142-05 could hang forever instead of failing. If anything panicked inside the scope before the
     folder watcher was stopped (the writer's final `fs::write(..).expect`, `writer.join().expect`, or the wait
     itself), the watcher's busy loop never stopped and `thread::scope` waited for it forever, with one CPU core
     at 100%. A wait that reads while letting others only read hung row 05 in 1 of 3 runs; one that keeps such
     a handle open hung it every time. Fixed in lane commit 1e4f826 ("green: t-142 lane row 05 guard and fixed
     state"): a `StopWatching` drop guard stops the watcher on any exit, and `rewrite_in_place` records a
     failed final write in `failures` instead of panicking.
  2. Low: "never creates the marker" was only tested under a missing parent folder, where it cannot happen,
     and a wait that writes back the partial bytes it read was caught only through the writer's sharing
     errors, in 4 of 5 runs. Fixed in the same commit by a fixed-state part: a short wait on an absent marker
     in a present folder, and on a planted partial line, must leave bytes, modified time and the folder
     snapshot unchanged.

  A note, no action: a wait that reports other text only at its limit, instead of at once, passes every row;
  the lane's cell does not require failing at once.
- **Migration re-review of the wording fix at 790091f: accepted, no blocker or medium finding.** Findings 1 and
  2 are fixed, checked site by site against the real t-141 lane copies: all 26 reworded sites print the old
  t-141 text with the marker path and the expected text, then `: {wait}` (in t-134 the prefix now shows its
  real trailing space); on a wrong line t-124 (twice), t-125 (`paused_run` and `paused_kill`) and t-131
  `paused_kill` now kill and reap the child, then fail the old assert_eq message plus exit code and stderr.
  The set of marker literals in the 8 files is unchanged, no `fn` or `#[test]` line changed, and
  `rustfmt --check` is clean. `MARKER_KEY_CRATES` pins exactly the set found by hand: 16 lanes, t-142 and the
  9 public suites. Lanes t-124, t-125, t-128, t-129, t-130, t-131 and t-132 passed 6 of 6 each, t-134 7 of 7,
  and t126 9 of 9 with all 85 lanes present; clippy counts on the 8 lanes match 1ac36e6, and every warning is
  on a line that already exists in the t-141 copies. Findings 3 and 4 stay open as known limits.

## Round 3 (2026-11-07)

The lane fix 1e4f826 entered the draft as 92eb03d (a cherry-pick), followed by the coverage-floor commits
558e26d (red) and 8d2c250 (green).

- **Private lane re-review at 1e4f826: accepted, no finding.** `StopWatching` is the first statement in the
  scope, so any panic in the two thread spawns, the wait or the writer join stops the watcher, and the writer
  has no `expect` or `unwrap` left. A wait that keeps the marker open now fails row 05 in 20 s with a verdict
  (it hung for more than 90 s in round 2). The fixed-state part checks a missing marker stays missing and a
  planted partial line keeps its bytes and modified time, with the folder snapshot unchanged; the real wait
  only reads, so it cannot flake, and the 50 ms gap is longer than Windows' file-time step of at most 15.6 ms,
  so a wait that writes always shows a new modified time. Only row
  05 changed since 64b2cf4, and none of its earlier checks was dropped. Six wrong waits (holding the marker
  open, writing back the partial bytes, reading while letting others only read, creating a missing marker,
  deleting a partial marker, opening it for writing) each fail row 05, none hangs, and all six rows fail on
  the red stub. The lane passed 6 of 6 at default threads (17.9 s) and at `--test-threads 1` (45.2 s), row 05
  alone 20 of 20; `rustfmt --check` is clean and `cargo clippy --all-targets` shows 0 warnings.
- **Shared wait and public checks re-review at 8d2c250: accepted, no blocker or medium finding.** The 13
  missed waits are caught: single raw waits planted at t-122:791, t-128:737, t-132:1382 and t-142:818 all fail
  `svev_015`. A raw poll before each of the 103 shared-wait calls is caught at the planted line 88 times and at
  the wrapper and its callers 5 times; the other 10 are the HT-142 test-made markers. A new shared wait on an
  unknown path, a new wrapper caller, an extra HT-142-05 call, a dropped row and a bogus row each fail. All
  20 `UNKEYED_SHARED_WAITS` rows are genuine test-made markers (the `svev_013` staged folder and HT-142-02 to
  05). The red reason of 558e26d is right ("the marker matcher misread 6 of 42 sources"), and on the base tree
  the 8d2c250 scanner lists waits in all 13 named lanes, the t-130, t-131 and t-132 lanes and all 9 public
  suites. t126 passed 9 of 9 and t125 4 of 4; `cargo fmt --check` is clean. Four low findings:
  1. When the marker is held in a struct field, a read through a crate-local helper is still not seen (a raw
     `while !slurp_marker(&held.marker).ends_with('\n')` added to t-122 passed `svev_015`), and the same holds
     for a field passed to a `wait_line` helper that reads through another helper and for a whole struct
     passed to a check helper the caller polls. The known-limits doc names only the one-shot read helper.
     Not changed; carried as a known limit.
  2. Rows are keyed `item::callee#N`, so a new call placed before an excepted call takes over that row: the
     check still fails because the count changes, but it reports the older, displaced call. Not changed;
     carried as a known limit.
  3. A new clippy warning, `needless_borrows_for_generic_args`, on `&format!(…)` at
     `t125_checkpoint_only_restoration.rs` line 2021 in `recover_case`. Fixed in round 4.
  4. No commit holds `svev_014` red; the reviewer rebuilt it. Carried as a known limit.

## Round 4 (2026-11-07)

- **Shared wait and public checks re-review of the lint fix: accepted, no finding.** Commit e959c3c ("green:
  t-142 t125 needless borrow") changes only one line of `test/qa/tests/t125_checkpoint_only_restoration.rs`,
  dropping the `&` before `format!` at line 2021 in `recover_case`. On a fresh clone at e959c3c, t125 passed
  4 of 4 (`wcpv_016`, `wcpv_017`, `wcpv_018`, `svev_014`), clippy on the ratmac-qa library and t125 shows 0
  warnings, and `cargo fmt --check` is clean.

## Round 5 (2026-11-07)

The first full proof passed, but the deliberate-fault pass after it found one fault no check caught: with the
three lines in `await_marker_line` that refuse text holding more than one line removed (`test/qa/src/support.rs`
lines 202-204), an exact wait still refused two lines by comparing text, but a prefix wait accepted
`holding root lock <path>\nsecond line\n`. The private lane's row `ht_142_02_input_routing` gained one case for
it, and the ticket's HT-142-02 attack column says so; that first full proof is superseded.

- **Private lane re-review of the new case: accepted, no finding.** Only row 02's doc comment and one case at
  its end changed in `test-hidden/t-142/tests/hidden.rs`, and only the prefix part of the attack column at
  ticket line 76. The case plants the control's first whole lock line followed by `second line\n` and expects
  the wait to fail reporting all 65 bytes it read. On a clone of the safety commit with the lane copied in, the
  lane passed 6 of 6; with only those three lines removed, 5 passed and only `ht_142_02_input_routing` failed,
  at `tests\hidden.rs:138:5`, "HT-142-02 attack: whole prefix line followed by a second line"; with the red
  commit's `support.rs`, all 6 failed with "the shared whole-line wait does not exist yet (SVE-005 red stub)",
  with no build error. The bytes are planted before the wait, so the case does not depend on timing; row 02
  alone passed 3 of 3 runs. `rustfmt --check --edition 2021` is clean. The reviewer noted, needing no change,
  that the mechanics sentence at schema.md line 1220 does not spell out the one-line condition, while the
  requirement at line 1214 ("its complete text, ending in its newline") and the ticket do.

## Round 6 (2026-11-07)

The second full proof passed, but the workspace quality run then failed `svev_015` in t126: its scanner reads the
private lanes, and the round-5 case added a fifth shared-wait call in `ht_142_02_input_routing` that no
exception row names, because the rows are keyed `item::callee#N` and only the first four exist. Neither the
drafter nor the round-5 review had run t126. t126 is unchanged; instead the new case and the existing
"another prefix" case now run in one loop around the fourth call, so its existing row still names it.

- **Private lane re-review of the loop: accepted, no finding.** Only row 02's doc comment and the end of its
  body differ from the last committed lane. Both cases are planted before the wait under the same prefix and
  limit, each expects the wait to fail reporting all the bytes it planted, and each keeps its own message; the
  loop matches ticket line 76. On a clone of the safety commit with the lane copied in, the lane passed 6 of 6;
  with only the three lines removed, only `ht_142_02_input_routing` failed, at `tests\hidden.rs:142:9`,
  "HT-142-02 attack: whole prefix line followed by a second line"; with the red commit's `support.rs`, all 6
  failed on the red stub with no build error. t126 passed 9 of 9, `svev_015` included. Written back as two
  separate calls, `svev_015` failed naming `ht_142_02_input_routing::await_marker_line#5`, and its output
  listed calls 1 to 4 under their existing rows, so the scanner does read this lane. Row 02 passed 2 of 2
  repeat runs and `rustfmt --check --edition 2021` is clean. The reviewer noted that because `svev_015` keys
  its rows by call count, any lane edit that adds or removes a shared-wait call needs t126 run before it is
  accepted.

## Known limits carried to the gap record

1. `svev_015`'s scanner does not see every way a test could wait on a marker. Unseen: a marker held in a
   struct field and read through a crate-local helper (including a one-shot read helper outside a wait loop),
   a field passed to a `wait_line` helper that reads through another helper, and a whole struct passed to a
   check helper the caller polls; and, as written in the known-limits doc above `marker_inventory`, a
   collection, `if let Some`, a closure parameter, a `concat!` key with a raw wait, `env::var`, `read_dir`,
   and a sleep followed by one `exists()` check. Markers a test makes itself, with no `RATMAC_TEST_*` setting,
   are outside the check by design.
2. `svev_015`'s exception rows are keyed `item::callee#N`, so a new call placed before an excepted call takes
   over that row and the failure names the older, displaced call. The suggested fix, adding the first
   argument's text to the row key, was not made.
3. The t-127 lane's `ht_127_04` keeps a local `while !ran_marker.is_file()` poll. It is allowed, because the
   file is written by `git config`, not named by a `RATMAC_TEST_*_MARKER` setting; no comment says so.
4. The "Replace …" list at schema.md line 1222 leaves out the t-130 and t-132 lanes and the already-whole waits
   that were also moved; schema.md line 1220 and the ticket's boundary list cover them.

## Settled by the landing commits

The draft's commit history did not prove two checks red (round 1 low 5, round 2 low 3, round 3 low 4): no
draft commit held `svev_014` red, and the committed `svev_015` red (67f012c) failed on 5 false positives
instead of the ticket's red reason. The draft is not landed as it stands. It lands as two commits built on
the t-141 landing: the red commit ("t-142: prove whole pause marker line failures") holds all three checks and
the stub, and each fails for its stated reason, recorded under `run-066`: `svev_013` with "the shared
whole-line wait does not exist yet (SVE-005 red stub)", `svev_014` because t125's `await_marker` returns the
planted empty marker at once instead of reporting it not yet published, and `svev_015` by listing 150 sites
in 34 files that wait without the shared wait. The green commit then carries every reviewed byte above,
except the 4-line t-141 assertion in t126 that landed after the review.

## Reviewed inputs

Every changed tracked file and every changed or new file of the private lanes, at the bytes the full proof runs on.

| Reviewed input | Tracking | SHA-256 |
| :--- | :--- | :--- |
| `.arca/log.md` | tracked, modified | `e7ab0cb945b0031a3d74c94c58e2ed2945f5e46c8bf3532067f6eedf11bd5e93` |
| `.arca/ticket/t-142.md` | tracked, modified | `e9384d0f7af3585820731dd19c01b93cc4038e74eef42821d5c0963bdc58a9b4` |
| `.ratmac/evidence/run-066/SVEV-013.toml` | tracked, new | `a44902f6dba17dd68f0b8d2d92a0ff9ba97da2cf90a879f55a4a331fd0aa72d2` |
| `.ratmac/evidence/run-066/SVEV-014.toml` | tracked, new | `df07dac24b413038d51dc4bbb65a8935a18abc0ea5b8f7af63da16d27131c38a` |
| `.ratmac/evidence/run-066/SVEV-015.toml` | tracked, new | `f8136623dec5a0dc832c5760345259987e529a5547c1f4eaf0ff0be6c36e9b88` |
| `.ratmac/lanes.toml` | tracked, modified | `051a405f34a9aa06412ce849df7ff6c38885ae6d0145214bda0f8dc04049d92a` |
| `test/qa/src/support.rs` | tracked, modified | `b7c5eac431e7b00f2ebfa926941fd17187660b55b3b7c3ddfec925ebcb1cc5db` |
| `test/qa/tests/t050_blocked_route.rs` | tracked, modified | `442f1f895d6c42898f4859eb4e3f86d4deab9fb5b2c04b654c2f94264c0c4331` |
| `test/qa/tests/t086_single_engine_binary.rs` | tracked, modified | `2894937e203d5bfa7bde57eda5145577b283ca39ce8843f3217f9f24b4545dc1` |
| `test/qa/tests/t122_successor_ownership.rs` | tracked, modified | `9eca47d601633d0bd1107013fa3cbb8fccb201a6f9919200c8cc6cf7c45af940` |
| `test/qa/tests/t125_checkpoint_only_restoration.rs` | tracked, modified | `b2aed7ad733f9d7a073c471035e9eac2f80ffa486ee23b567efe69670349339a` |
| `test/qa/tests/t126_shared_private_test_support.rs` | tracked, modified | `4e4c0e0dddab61cef0e7a03fd026cb2e46826155ded8dde1a3fddc647537b4a5` |
| `test/qa/tests/t127_edition_proof_preflight.rs` | tracked, modified | `7cc47d25a17b55a4ac8a690afdccd46dc089bf8c60999a4d8f8cc9b9fd2c64f7` |
| `test/qa/tests/t128_deterministic_trace_channel.rs` | tracked, modified | `b61c7bb7f79a5eb91339e22add8264d9bc26038cda4854057bd7dd5e049638ca` |
| `test/qa/tests/t130_cycle_pause_routes.rs` | tracked, modified | `105ff5d9106199a56cb177ede9dcde0d0f206747be74b7aadc13f231244f8e31` |
| `test/qa/tests/t134_safe_nested_invocations.rs` | tracked, modified | `2c9690b4e3286c6eaa64944e7fb560ef813303a2bab8f6f933bca61b2ed1e622` |
| `test/qa/tests/t136_lock_contention.rs` | tracked, modified | `5ffbd9c57d364212b7239c28ea9ff02799c657e17f01fe24d4eb23e627aa75fe` |
| `test-hidden/t-073/tests/hidden.rs` | lane, ignored by Git, modified | `6e42c57002cbc787eef5540525c590670599b72c9e598bd691e54732da3e6970` |
| `test-hidden/t-080/tests/hidden.rs` | lane, ignored by Git, modified | `a2cd6b609ce9b24ccaa8dc836f507cce6dcf8e448247e27e9765fa755a200fed` |
| `test-hidden/t-081/tests/hidden.rs` | lane, ignored by Git, modified | `fb031ce1466d2dd0c19410252d5758d99003e2753f481336c4f88148f542151b` |
| `test-hidden/t-086/tests/hidden.rs` | lane, ignored by Git, modified | `8e255dc0d5cb23d7fc42601b76bf906f5ff0df289c41c0c77bccda23e0aaa266` |
| `test-hidden/t-087/tests/hidden.rs` | lane, ignored by Git, modified | `5cfa11a5f480408d33aeebfe7c91aee6c545b81b229f4c0251dff804f00443bf` |
| `test-hidden/t-122/tests/hidden.rs` | lane, ignored by Git, modified | `59835e5f7b23febf3098116cff53812644eb7e1e8464071cf34f22a18f1ac60d` |
| `test-hidden/t-124/src/lib.rs` | lane, ignored by Git, modified | `f195f0b69e2d9d0126f980a887e29a9a00abeb9ab7900b09cfdabf46f51e7376` |
| `test-hidden/t-124/tests/hidden.rs` | lane, ignored by Git, modified | `81af0e7f534ac3172f71cce4f006ff2a3ddb36b95ec297e2f818a9a86da8068d` |
| `test-hidden/t-125/src/lib.rs` | lane, ignored by Git, modified | `7ee0360b35c88404e126fa2d6c596e722c0ce26121bb447d52c41aaf01aed9f9` |
| `test-hidden/t-125/tests/hidden.rs` | lane, ignored by Git, modified | `e71a4fb750662cfc0d95616b625c49d852b3c0d20388103062fb9c015a84d76d` |
| `test-hidden/t-127/src/lib.rs` | lane, ignored by Git, modified | `7d7d8b096ea2131e37d6658f3a0e8dde7939245543e468d6b3106e08de2d9b7a` |
| `test-hidden/t-127/tests/hidden.rs` | lane, ignored by Git, modified | `aaca7647fa611b53d865644c6d72afef845c562767009fa0aba98b848df98d53` |
| `test-hidden/t-128/src/lib.rs` | lane, ignored by Git, modified | `f5179852d4148e9fa3f0276ed77e210a20cc2e774314760783201da83c0e683a` |
| `test-hidden/t-128/tests/hidden.rs` | lane, ignored by Git, modified | `bf97f2017136c79f421012bcc003e55fdd8462d69e9897d6ea6cb5da81bbb505` |
| `test-hidden/t-129/src/lib.rs` | lane, ignored by Git, modified | `0b95ac9398dff7c95280b5df44dfb5c384fc9eecf8c93ed16aad74bbbf58d159` |
| `test-hidden/t-129/tests/hidden.rs` | lane, ignored by Git, modified | `7a18d5f310760de65d0f0cfafad571424f6f6652748943894b77dab1f7e4e8da` |
| `test-hidden/t-130/src/lib.rs` | lane, ignored by Git, modified | `5bf8e4c911dc64063dbc5e6cede4fee9e9e010d234cf33c7ee04c301da9a811f` |
| `test-hidden/t-130/tests/hidden.rs` | lane, ignored by Git, modified | `70300a666dd3d3368f8d4215187c2c598e2c2ef6bcad79b291168a36ed55bf91` |
| `test-hidden/t-131/src/lib.rs` | lane, ignored by Git, modified | `d1b5d234d257285620c21279289d58b3324156e44ad457a959e1a66474bdcce0` |
| `test-hidden/t-131/tests/hidden.rs` | lane, ignored by Git, modified | `3d6d34d26111a26f5241dd44ed2fdf943bf3497423ab41ed5c6c1e7ea0d5d07e` |
| `test-hidden/t-132/src/lib.rs` | lane, ignored by Git, modified | `59516bce2f248596234d8b410a84b3cea5c7e65f02b0c99a648653993c321c69` |
| `test-hidden/t-132/tests/hidden.rs` | lane, ignored by Git, modified | `ec8d2324783d180184daa4d22a20a872fb7a5ed95edda9a2684d15d367794c3b` |
| `test-hidden/t-134/src/lib.rs` | lane, ignored by Git, modified | `e62a307af3e4cc7c265f8cd033891dc2c63466155817c9964b298ff99034c86d` |
| `test-hidden/t-134/tests/hidden.rs` | lane, ignored by Git, modified | `833de8da35d71e28bd5f494bdb94e5cf21f37482b39fa9b69c21e8938ac6337e` |
| `test-hidden/t-136/tests/hidden.rs` | lane, ignored by Git, modified | `da3e530c60cb0222c9a4732825a90cb6b7b3af0a5548d3b82e95867d3ed1057b` |
| `test-hidden/t-142/Cargo.lock` | lane, ignored by Git, new | `a2205ef247eab8d9e7ea75b4f71e6485a9939d89e1bc931e5916256b3a65d7a5` |
| `test-hidden/t-142/Cargo.toml` | lane, ignored by Git, new | `5e7627b6632deb8f408e4d842944dff6e6050b6c1ad41011953a8de148d14e8c` |
| `test-hidden/t-142/src/lib.rs` | lane, ignored by Git, new | `f575c8ce69bc15b7850d4b3383b34c70c10f6f2a05361dadb331dab02c908f39` |
| `test-hidden/t-142/tests/children/mod.rs` | lane, ignored by Git, new | `a224f8ab253304a96d8973edcc2b6be2340a1ceef9d7e41478e79e045b2442cc` |
| `test-hidden/t-142/tests/governed/mod.rs` | lane, ignored by Git, new | `2142b7b479022f3fcbf6963d54b9aada0a881c3e7eac4cd06e83b4b8f3cdd584` |
| `test-hidden/t-142/tests/hidden.rs` | lane, ignored by Git, new | `6fdaa8209f99d78da875938cdf484fe65269caa2bf4298aad53609848ce6420d` |
