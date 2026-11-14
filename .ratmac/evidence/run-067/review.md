# Independent review before full proof

Ticket [t-143](../../../.arca/ticket/t-143.md), Run `run-067`, requirement SVE-006 (case-scoped test hooks).
The independent reviewer built and ran tests only in its own throwaway clone with its own build folder; it
edited no file and ran no git command that changes state in the draft or in either real checkout, and it
left no scratch folder behind. The drafter reported two slips of the coordinator's command rules, both in
read-only commands.

## Round 1 (2026-11-07)

Before the independent review, three helpers of the drafter each read one part of the draft and approved it.
They are not independent, and nothing below rests on them:

- **Scanner shapes: approved.** The four source shapes the `svev_018` scanner must read, each with its control,
  and the t-142 lane's `launch` setter, which sets its keys through a loop variable.
- **Private lane: approved.** Every row HT-143-01 to HT-143-06 has its valid control and its attack, and the
  crate builds the Engine library with the `test-fault-injection` feature, so an in-process command does pass
  through the hold barrier.
- **Green change: approved.** All six barrier-armed `hold` and `resume` commands of `wrsv_003_08` run as a
  child through `armed`, all 43 lines that set or removed a barrier setting for the whole test process are
  gone, unarmed commands stay in-process, and t119 and t120 only swap their private mutex for
  `support::hook_lane()`.

The independent reviewer then read the first green draft ("green: t-143 implementation") and built wrong
variants against it. Its round-1 report was not kept; what follows is what its round-2 report restates as
found and fixed:

1. HT-143-04: the pending hold is now finished by `resume` and then `step`. A lane that arms the barrier for
   the whole test process (variant L1 below) now fails the row at `tests/hidden.rs:798` with
   `resume: committed with recovery pending: hold test barrier expired before after-journal-publication`.
2. One builder: `armed_command` now serves both `Armed::spawn` and `armed_environment`, and HT-143-06 now
   requires exactly the four barrier settings and no `StepFault` setting, so a builder that starts its child
   unscrubbed (variant L4) fails at `tests/hidden.rs:965`. An implementation that goes around the builder is
   still not caught.
3. The scanner excepted `cli::help` by name. The exception is removed, and t120's `webv_016` takes
   `library_lane()` like every other in-process Engine case.
4. Ticket line 53 now says how HT-143-05 reads "the same files with the same bytes": each fixture's own root
   path, and the `held-context-sha256` value framed from it, are replaced by placeholders before the bytes are
   compared. This matches the lane's `project_held_context`.

The fixes are the draft commits "red: t-143 review checks" and "green: t-143 review fixes".

## Round 2 (2026-11-07)

**Accepted, one low finding, no medium or blocker**, at draft commit 5ed973d. All fingerprints the
coordinator gave matched.

- **Low:** the ticket's line-53 edit was in the green commit under the heading "Settled before red", so at the
  red commit the ticket still named the `cli::help` exception that the red t126 had already removed. Fixed by
  moving the edit into the review red commit: the draft now ends d86c0ea ("red: t-143 review checks") and
  42edb74 ("green: t-143 review fixes"), and every file at 42edb74 has the bytes reviewed at 5ed973d.

What the reviewer checked:

1. **Round-1 fixes** are all in place; it rebuilt the wrong variants itself (table below).
2. **`svev_015` after the rebase onto t-142** passes at all three draft commits. t-143 adds no exception row;
   the only table change is the `"test-hidden/t-143"` entry in `MARKER_KEY_CRATES`. The lane's two raw marker
   reads in HT-143-02 became `await_marker_line` with `MarkerLine::Exact` at a zero limit, which is as strict
   as the old byte check. With the marker setting removed from t130's `armed` and from the lane's
   `armed_command`, `svev_015` flagged t130 `:3469`, lane `lib.rs:534`, and `hidden.rs:591`, `:600` and `:894`,
   so the waits are traced, not hidden.
3. **t-142's race wait is no weaker.** It still requires the exact whole line
   `holding before after-journal-publication` through `support::await_marker_line`. Its deadline went from
   4 s to 15 s to allow for starting a child process. Assertion counts before and after: t119 95 and 95, t120
   158 and 158, t130 307 and 319, the t-130 lane 117 and 117, t126 108 and 113. The only lines removed are the
   two direct waits, now inside the helper, and one thread join.
4. **`svev_018`** passes at green and fails at the review red commit on exactly
   `Engine case without the lane: t120:2982 webv_016`, with `webv_013`'s armings reported beside it. With each
   of the 65 lane-taking lines removed one at a time, 55 removals are caught. The other 10 do not need the
   lane: 6 in t130 and 2 in t133, binaries with no setting armed for the whole process left, and 2 inside
   isolated children (the t-128 lane `:1109`, the t-143 lane `:1085`).
5. **Green counts:** t119 4 of 4, t120 4 of 4, t126 10 of 10, t130 11 of 11, the t-130 lane 6 of 6, the t-143
   lane 7 of 7. Formatting is clean for the workspace and the lane. Clippy warns only in t113 and t135, which
   t-143 does not touch.

Wrong variants, all built on 5ed973d. F: the check fails, so it catches the variant; pass: the variant slips;
blank: not run.

| Wrong implementation | svev_016 | svev_017 | svev_018 | HT-01 | HT-02 | HT-03 | HT-04 | HT-05 | HT-06 cross | HT-06 child |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| W1: t130 `armed` also sets the barrier for the whole test process while its child lives | F `:3564` | F `:3665` | F | | | | | | | |
| L1: the lane's `Armed::spawn` arms the whole test process and starts its child unscrubbed | | | | F `:504` | F `:243` | F `:209` | F `:798` | F `:883` | F `:209` | pass (trivial) |
| L4: the lane's builder starts its child unscrubbed | | | | pass | pass | pass | pass | pass | F `:965` | pass |

Shapes the reviewer planted on the real tree that `svev_018` does not flag, with one control it does:

| Shape | Planted at | `svev_018` |
| :--- | :--- | :--- |
| `drop(_lane)` before a real arming | t132 `wrsv_004_03` | slips |
| Lane taken only in an inner block | t132 `wrsv_004_07` | slips |
| A `set_var` for the whole process after the isolated block | `ht_143_06_isolated_child` | slips (counted as isolated scope) |
| `Fixture::routed(..).run(..)` on a call result | the t-130 lane | slips |
| The same call through a named local (control) | the t-130 lane | caught |

No site in the tree today has one of these shapes, so the reviewer asked for no coverage floor.

## Known limits carried to the gap record

1. `svev_018`'s scanner does not follow a method called on a call's result, such as
   `Fixture::routed(..).run(..)`.
2. Once a case takes the hook lane, the scanner counts the lane as held for the whole case: an explicit `drop`,
   a lane taken only in an inner block, and a closure are not tracked.
3. A case that calls `isolated_test` with its own name is trusted as a whole, including the code outside its
   child branch.
4. A child that runs one named case (`--exact`) is recognised only when its branch is written as a braced
   `if` or `match` arm.
5. The scanner uses the drafter's rules of thumb for the first variable of a tuple pattern and for helper
   names.
6. The t-143 lane proves its rows through one builder, `armed_command`; an armed command built around it is
   not caught by the lane.

## Settled by the landing commits

The draft is not landed as it stands. It lands as two commits built on the t-142 landing. The red commit
("t-143: prove case-scoped test hook failures") holds all three checks, a red helper in t130 that sets the
barrier for the whole test process, the roster line and the ticket's "Settled before red" paragraph. Each
check fails there for its stated reason, recorded under `run-067`: `svev_016` because a hold with no barrier
of its own is refused with "hold test barrier expired before before-journal-publication waiting for" the
armed hold's release file; `svev_017` because all four `RATMAC_TEST_HOLD_BARRIER*` settings are visible in
the test process and in a plain child; and `svev_018` by listing 81 of 179 hook places, 76 in the public
suites and 5 in the t-130 lane, which at red does not yet hold the lane in `ht_130_02_input_routing` and
`ht_130_03_lifecycle_model`. After the green commit every reviewed file has its reviewed bytes: t119, t120
and t130 and the five lane files equal the draft at 42edb74 byte for byte, and t126 differs from it only by
the 4-line t-141 assertion that landed after the draft was cut.

## Reviewed inputs

Every changed tracked file and every changed or new file of the private lanes, at the bytes the full proof runs on.

| Reviewed input | Tracking | SHA-256 |
| :--- | :--- | :--- |
| `.arca/log.md` | tracked, modified | `70575183440f73d7e9f04b6e552f7d2f00064d4d615405a9f1726a3cadfb8ef7` |
| `.arca/ticket/t-143.md` | tracked, modified | `9ddb4cfa9a6fd2800549d4421cc4f0026852292c39d5e7b8b333cd43f8c1a5a0` |
| `.ratmac/evidence/run-067/SVEV-016.toml` | tracked, new | `76087c293c238864b1276daf69eef4573250a127bef2d936572ad945c9bc72f5` |
| `.ratmac/evidence/run-067/SVEV-017.toml` | tracked, new | `2e2272743f8b670b04d1d1e6c767adbb01fe06cf3bce3f2e31acef96da1a8088` |
| `.ratmac/evidence/run-067/SVEV-018.toml` | tracked, new | `3c2922178cc8794b520ccba93300128406385fc9e8b64bfb472cbdc5605d8532` |
| `.ratmac/lanes.toml` | tracked, modified | `7a279bf70f054147ea36dc160370bc35e621ebb33cdf9ca8caf55e0e19e8f493` |
| `test-hidden/t-130/tests/hidden.rs` | lane, ignored by Git, modified | `173c0b6d66648380e2cff737ff3092b793c9e6294d31a2e62b1206458f33b160` |
| `test-hidden/t-143/Cargo.lock` | lane, ignored by Git, new | `8cb29157ed0131bb93a2bb7d9cb5a3264c1450902decd8cb5363c54d7fb5fefc` |
| `test-hidden/t-143/Cargo.toml` | lane, ignored by Git, new | `f2847902fc233b46f3e8142ea8deb7ff9b4a0d7da5958563854e5873c9b5fb7f` |
| `test-hidden/t-143/src/lib.rs` | lane, ignored by Git, new | `8afe47e81cb5cc4301e4549c2d858091ac7a8ef122499b61f1328c5326947419` |
| `test-hidden/t-143/tests/hidden.rs` | lane, ignored by Git, new | `c168c40dadfeee38a2e65faabc5dd6ece210e2582919f2ab70b688b226f3e2be` |
| `test/qa/tests/t119_single_root_resolution.rs` | tracked, modified | `4f894be9b0ef18933a1b2c4b224419297a813394549c621f3a9393259d309fd2` |
| `test/qa/tests/t120_residue_refusal_precedence.rs` | tracked, modified | `729917de4e1d9d1fc79e3eaf057b6706ff1b9adec4a08520c4795a82ef7994da` |
| `test/qa/tests/t126_shared_private_test_support.rs` | tracked, modified | `d260139cc84b56d82980c567f829e94f83041ff263a5d1686dc7e013ec0e476b` |
| `test/qa/tests/t130_cycle_pause_routes.rs` | tracked, modified | `621d27941a834ff9bfa20a34e763cc7536f140f695e01ddef16d0f4d5e4c0930` |
