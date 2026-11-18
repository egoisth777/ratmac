# Independent review before full proof

Ticket [t-150](../../../.arca/ticket/archive/t-150.md), Run `run-074`, requirement WTC-005
(endpoint release window). Independent review of the current green source, the three public checks,
the t126 inventory row, and the four private-lane files against the whole ticket, WTC-005 in the
goal design, the accepted issue, and SVE-005 through SVE-007. The reviewer authored none of those
inputs. Tests, builds, interface probes, and wrong implementations ran only in throwaway copies,
with separate build folders and fixture roots; no Engine lifecycle command ran against a real
checkout. No source, test, contract, or receipt was edited in either real checkout. This record is
the reviewer's only real-checkout write. All input hashes below matched the original scratch copy
at the end of the review. The scratch executables were no longer running before scratch deletion.

## Round 1 (2026-11-18)

**Rejected; no blocker, four medium findings and one low finding.** Reviewed red commit
`ba65f9e2e2cfe4331754eb810495b97740bf0b2a`, on `253a063`, plus the uncommitted green changes to
`src/endpoint.rs` and `src/nested.rs`. Passing standalone checks do not clear the findings below.

### Findings by severity

- **Blocker: none found.** Both endpoint retry loops retain their existing answers and five-second
  bounds. `RunLock::try_acquire`, the fresh mint's single try in `src/scheduler.rs`, and the lock module
  are unchanged. The actual green implementation places its new pause after starting the bound.
- **Medium 1 — the plain root debug binary contains all four new hook names.**
  `cargo build --offline -p ratmac --target-dir <private-output>` succeeded without the fault feature,
  but the resulting `debug/rtm.exe` contains `RATMAC_TEST_ENDPOINT_RETRY_PAUSE`,
  `RATMAC_TEST_ENDPOINT_RETRY_PAUSE_MARKER`, `RATMAC_TEST_ENDPOINT_RETRY_PAUSE_RELEASE`, and
  `RATMAC_TEST_ENDPOINT_RETRY_PAUSE_TIMEOUT_MILLIS`, as well as `endpoint retry` and `terminal-read`.
  `src/nested.rs:969-975` leaves `ENDPOINT_RETRY_PAUSE` ungated and the production no-op callers
  still receive that constant. The environment-reading implementation is feature-gated and a
  direct production probe confirms the seam is inert; that does not satisfy this review's explicit
  requirement that a normal root build carry none of the new names. A separate optimized release
  build eliminates them. The old pause names also occur in a plain debug build of the pre-ticket
  source; that precedent does not account for these newly introduced names. Gate the new constant
  and its production references, rather than relying on release optimization.
- **Medium 2 — HT-150-03 can reject the reviewed green solely because its fixture runs slowly.**
  `test-hidden/t-150/tests/hidden.rs:961-972` starts a five-second fixture clock before spawning the
  endpoint, waits for its marker, sleeps 1.5 seconds, and collects the peer before asserting the
  elapsed time is below five seconds. Process startup and peer collection consume this allowance.
  The unchanged green lane passed alone (6/6, 48.20 s), but one of four concurrent independent lane
  test processes failed at line 969, `fixture precondition: the claim window closed inside its bound`;
  its other five tests passed, as did all six tests in each of the other three processes. The failed
  process completed in 94.48 s. No wrong implementation was present. Several initially concurrent
  mutant runs hit the same fixture assertion; those failures are not credited as mutation catches.
  The test must arrange or explicitly invalidate/retry a missed fixture window instead of treating
  scheduler delay as a product failure. A similar narrow interval exists at lines 989-1000.
- **Medium 3 — the lane kills its direct child, not the endpoint process tree.**
  `Spawned::Drop` at `tests/hidden.rs:192-199` calls `Child::kill` and `wait` only on the parent step.
  An endpoint spawned by that step is a separate process on Windows. Under V19, after the lane had
  returned its verdict, PID 63456 was still running the lane's `rtm.exe endpoint` from the reviewer's
  `vw/v19` copy; four fixture directories remained. The process later exited at its own seam timeout,
  but it was not reaped by the case that launched it. Other failed cases also left fixture folders;
  the green concurrent HT-150-03 failure left its `ratmac-t150-l03-both-calls-*` directory. By contrast,
  the public `WindowStep::stop` uses `taskkill /T /F` and then waits. Give the lane equivalent owned
  tree cleanup on assertions and expiry, before dropping fixture trees. Every public mutant run
  left no processes or fixture directories in its owned temp root.
- **Medium 4 — HT-150-03 does not prove two of its settled first-retry properties.**
  V02 starts each five-second bound only after returning from the pause, and V03 pauses on every
  retry rather than the first. Both pass all three public checks and all six private rows. The lane's
  two-window test proves separate call budgets (V13 is caught), but does not prove that time spent
  paused consumes the budget. Tests ordinarily close the peer window before releasing the endpoint,
  so another retry either never occurs or sees the still-existing release file immediately; repeated
  pauses are not exposed. The settled paragraph and HT-150-03 require the bound to start at the first
  error and the first-retry pause once per call. Add a held-open-window case that spends the budget
  paused, and an observation that distinguishes a later retry from the first. Independent direct
  probes establish that the reviewed source itself gets the pause-budget rule right.
- **Low 1 — additional wrong implementations survive every named check.** V04 (only the first
  denied `terminal_snapshot` call pauses), V10 (remove pre-admission retry-companion validation), V17
  (wrong retry expiry text), V18 (comma-list matching leaks into old exact-value seams), and V20
  (a live claim holder is retried to the bound instead of refusing at once) all survive. These are
  proof gaps, not descriptions of the reviewed green source. V10 still validates when a pause is
  actually entered; without a lock error the missing companions no longer refuse before admission.
  HT-150-01 checks the old endpoint seam's companions, not the new retry seam's. V18 was also run
  against the whole t134 suite, which still passed. See the matrix and known limits.

### Source and fixture review

The green diff adds first-retry calls at `src/endpoint.rs:323` and `:760`, a shared wrapper, and
pre-admission validation at `:543`; the original acquire calls, held-lock answers, error answers,
retry sleep, and deadline checks remain. Each call has its own local deadline. Only the feature
implementation reads environment variables or waits. The production-name exception is Medium 1.

`arms(..., true)` splits the new selector on commas and recognizes the exact tokens `claim` and
`terminal-read`; non-place tokens are ignored. No recognized place means no validation or pause.
`run` validates recognized companions before admission. The marker is replaced with `<place>\n`
using `fs::write`; the release file is observed but never removed; expiry renders
`rtm: endpoint: injected endpoint retry pause at <place> expired`. Old NESTED_PAUSE, ENDPOINT_PAUSE,
and skill callers use the exact-value path (`arms(..., false)`), not the listed path.

Independent probes on the green harness Engine confirmed missing marker, missing release, missing
and invalid timeout (zero and nonnumeric), a recognized token mixed with a non-place token, wholly
unknown/empty selectors, exact claim expiry text, replacement of a prefilled marker, and retention
of a pre-existing release file. Missing companions refused before a child Run or launch appeared.
For each place, keeping its release window open and holding its retry pause for 6.5 seconds produced
the open-error refusal about 9 ms after releasing the pause, not a new five-second wait. These probes
are review evidence; they are not part of the public suite or declared private lane.

The public fixture reaches an already-terminal child by an interrupted after-terminal endpoint.
Its peer is the child's own `rtm step`, using the real lock module's `unlink` hold on Windows; no
synthetic open-denial fault replaces the window. WTCV-014 arranges the peer while after-admit is
parked, so it obstructs the terminal read rather than the claim. WTCV-015 independently checks a
live holder and a lasting release window. Whole-marker synchronization, not a race to close the
window, makes the three public checks fail under `l18`.

| Check or row | Fixture/oracle review |
| :--- | :--- |
| WTCV-013 / `wtcv_013` | Unobstructed terminal-resume control; real claim window; exact `claim\n`; close peer then release endpoint; identical output and one child admission. Meets its P4 cell. |
| WTCV-014 / `wtcv_014` | Recording driver control; after-admit park; real terminal-read window; exact `terminal-read\n`; identical output, one admission, no extra driver launch. Meets its P4 cell. |
| WTCV-015 / `wtcv_015` | Live-holder exact refusal without retry marker; separate lasting unlink window with marker, at least five seconds from parent start, exact open error, unchanged parent and child roster. Meets its P4 cell; it does not impose an upper bound on live-holder refusal. |
| HT-150-01 / regression | Fresh and interrupted pre/post-terminal children, plain/listed/unknown retry selectors, existing endpoint boundaries and companion refusals, literal output and durable comparisons. Does not exercise new-seam companion refusal or comma values for old seams. |
| HT-150-02 / input-routing | Both places with the opposite selector and then both selectors; whole marker names, receipt identity, and driver count checked. Catches wrong-place routing and non-listed selectors. |
| HT-150-03 / lifecycle-model | Two independent windows in one resume, marker replacement/rearm, one receipt/admission/transition, plus late-closing windows. Catches shared budget, appended markers, and Engine release removal. First-error budget/once-per-call proof and timing reliability remain Medium 2 and 4. |
| HT-150-04 / durability-recovery | Live and lasting refusals preserve parent/child bytes and launch count; resumes after peer exit; kills unlink peers at each place and requires completion. Direct-child cleanup is insufficient on case failure (Medium 3). |
| HT-150-05 / output-filesystem | Unobstructed/window twins at each place, exact stdout/exit and both refusals, receipt digest and pins, exact marker line. Wrong retry expiry text is outside its exercised paths. |
| HT-150-06 / cross-feature | Current-child receipt identity/consumption once, repeated receipt not consumed, separate unarmed fixture completes while armed terminal-read endpoint waits. Covers case isolation and safe nested receipt behavior. |

SVE-005: every marker wait introduced here uses the shared `await_marker_line` directly or through
`await_pause_line`, `Pause::expect`, `expect_replaced`, or `absent`; no file-exists/partial-read marker
wait was found. HT-150-03 deliberately accepts the earlier whole line while waiting for replacement,
then requires exactly the new whole line. SVE-006: settings are added only to owned child `Command`s;
shared `support::command` scrubs inherited hook settings. No process-wide hook mutation was found.
SVE-007: the lane has an independent workspace and Cargo output and launches its own Cargo harness
Engine, not a repository-shared executable. The t126 inventory row and roster line include t-150;
t126 passes on the reviewed tree.

### Commands on the reviewed green bytes

All Cargo commands used `--offline`; test commands additionally used `--locked`. The first full
set ran in one private copy with no mutant overlay. The later green variant and quiet/concurrent
runs used byte-identical source and lane files in other private copies.

| Command/check | Result |
| :--- | :--- |
| Root `cargo build --offline -p ratmac` with private target directory | Exit 0; no fault feature; binary-name audit fails (Medium 1) |
| Root optimized release build with private target directory | Exit 0; new names absent |
| `cargo test --offline --locked -p ratmac-qa --test t134_safe_nested_invocations -- --exact wtcv_013 wtcv_014 wtcv_015` | 3 passed, 0 failed |
| `cargo test --offline --locked -p ratmac-qa --test t134_safe_nested_invocations` | 7 passed, 0 failed |
| `cargo test --offline --locked -p ratmac-qa --test t126_shared_private_test_support` | 10 passed, 0 failed |
| `cargo test --offline --locked -p ratmac-qa --test t136_lock_contention` | 6 passed, 0 failed |
| `cargo test --offline --locked --manifest-path test-hidden/t-150/Cargo.toml` | 6 passed, 0 failed; unit/doc targets also passed |
| Green lane executable alone, then four independent copies concurrently | Alone 6/6; concurrent results 6/6, 5/6, 6/6, 6/6; the failure is Medium 2 |
| `cargo fmt --check`; `cargo fmt --all --check`; lane Rust files via `rustfmt --check --edition 2021` | Exit 0 |
| `python tools/check_links.py` | Exit 0 |

### Wrong implementations tried

Every variant was newly built in its own copy, with its own root and lane build output. Each ran
all three public checks and all six private rows unless stated otherwise. `013` means `wtcv_013`,
etc.; `02` means HT-150-02, etc. `None` means all relevant tests passed, not untested. Initial mutant
runs overlapped; whenever HT-150-03 failed its fixture deadline, the affected lane was run again
alone before assigning a catch. The matrix below uses the uncontaminated results. All public
results come from correctly filtered, three-test runs; an earlier runner bug that selected zero
tests was identified and those empty runs were discarded.

| Variant / wrong implementation | Public checks that failed | Private rows that failed |
| :--- | :--- | :--- |
| V00: red source, no retry seam | 013, 014, 015 | 02, 03, 04, 05, 06 |
| V01: `l18`, release window in `RunLock::try_acquire` becomes `Ok(None)` | 013, 014, 015 | 02, 03, 04, 05, 06 |
| V02: pause before starting each call's bound | None | None |
| V03: pause on every retry, not just the first | None | None |
| V04: pause only in the first denied `terminal_snapshot` call | None | None |
| V05: new selector exact-only, no comma list | None | 02, 03, 04, 05, 06 |
| V06: append marker line instead of replacing | None | 03 |
| V07: Engine removes retry release file | None | 03 |
| V08: claim pauses once but returns first open error without retrying | 013, 015 | 02, 03, 04, 05 |
| V09: terminal-read `Err` returns `Ok(None)` | 014 | 02, 03, 04, 05, 06 |
| V10: omit pre-admission retry-companion validation | None | None |
| V11: any recognized new selector pauses at either place | None | 02 |
| V12: marker records selector list instead of actual place | None | 02, 03, 04, 05, 06 |
| V13: share one deadline across endpoint places | None | 03 |
| V14: terminal read pauses once but returns first open error without retrying | 014 | 02, 03, 04, 05, 06 |
| V15: replace both five-second bounds with one second | 015 | 03, 04, 05 |
| V16: validate companions even when new selector has no place | None | 01 |
| V17: retry expiry says `timed out`, not `expired` | None | None |
| V18: split comma lists for old exact-value seams too | None (whole t134 also passed) | None |
| V19: pause before any denied lock try | 015 | 01, 02, 04, 05, 06 |
| V20: live claim holder waits to the bound before held refusal | None | None |

V01 and V00 fail all three public checks waiting for the missing first-retry marker, not because a
release was closed too soon. For V06 the second line cannot equal the exact replacement line; V07
fails rearming the release file the Engine wrongly removed. V13 reaches both windows but refuses
with the stale shared open-error deadline. V09 and V14 catch terminal-read retry loss independently
of the claim. V10's initial HT-150-03 failure was the unrelated fixture precondition; alone all six
rows passed, so no catch is claimed. Seven wrong variants survive every named public/private check:
V02, V03, V04, V10, V17, V18, and V20.

## Round 2 (2026-11-18): the repaired green and private lane

**Accepted; no blocker, medium, or low finding, and no surviving wrong implementation**, at the same
red commit `ba65f9e2e2cfe4331754eb810495b97740bf0b2a`, on `253a063`, plus the uncommitted green
changes to `src/endpoint.rs` and `src/nested.rs` and the four lane files hashed under Reviewed
inputs. Only three inputs changed after round 1: `src/endpoint.rs` (round 1
`2494641973dc885b8ad2f38f16a04b8b44b07d215ac7259dfa8b016b751b5486`), `src/nested.rs` (round 1
`74bcf9dbd7a68154fff07f86541778a7276613ee98bfec37246630b22249bb78`), and
`test-hidden/t-150/tests/hidden.rs` (round 1
`bc7ee9f9eb7ce7991830328212d7b6e00225fc5ed7db408bd900b2ea7f926877`). The public checks, the t126
row, the roster line, the receipts, the ticket, the log, and the other three lane files are
byte-identical to round 1; `src/lock.rs`, `src/scheduler.rs`, `test/qa/src/support.rs`, and
`test/qa/tests/t136_lock_contention.rs` are unchanged from `253a063`. Round 2 used its own fresh
scratch copy, deleted at its end like round 1's.

### Round 1 findings, rechecked

- **Medium 1 is closed.** `ENDPOINT_RETRY_PAUSE` and every new function that reads it (`arms`,
  `validate_companions`, `hold_pause`, `hold_endpoint_retry_pause`, and the endpoint's `retry_pause`
  mapping) compile only with `test-fault-injection`; a plain build gets no-op `retry_pause` and
  `validate_endpoint_retry_companions`. A plain root `cargo build --offline` with a private target
  directory exited 0 without a warning, and its `debug/rtm.exe`
  (`0ead458af3595d82b25c5fc93b7a1980dde646a92d0caaa8af461e20213580da`) contains none of the four
  `RATMAC_TEST_ENDPOINT_RETRY_PAUSE*` names, `endpoint retry`, or `terminal-read`; the
  fault-feature harness `rtm-qa.exe` contains each of them. The older pause names still occur in a
  plain build, as they did at `253a063`.
- **Medium 2 is closed.** The cases whose verdict needs a real window to close inside its bound now
  run through `qualifying_fixture`. An attempt whose conservative start (taken before the endpoint
  can first be denied) is too old prints `fixture scheduling miss`, has its endpoint tree killed,
  and is retried, up to twelve times, after which the row fails `fixture unavailable`; a miss never
  passes a row. The unchanged green lane executable (`hidden-5ddafe0a0c889f47.exe`) passed alone
  (6 of 6 in 48.18 s, no miss), as four concurrent copies twice (24 of 24 each time, no miss), and
  as six concurrent copies (36 of 36; each copy logged two late-window misses, both retried and
  then qualified). No run anywhere in round 2 ended `fixture unavailable`.
- **Medium 3 is closed.** `Spawned::stop` ends the whole process tree (`taskkill /T /F` on Windows;
  a process group and `kill -KILL -- -<pgid>` elsewhere) before reaping, and `Spawned`, `Hold`, and
  `Pause` stop or release on drop. After each of the 23 lane runs in the matrix below, 22 of them
  failing, no process remained under that variant's copy and its temp folder was empty, at once and
  five seconds later; the same held after all fifteen stress copies.
- **Medium 4 is closed.** HT-150-03 holds an armed pause past the bound with the real window still
  open, closes the window, then releases the pause, and requires today's open-error refusal with no
  durable change ("paused time consumes the first-error budget"); a bound started after the pause
  (V02) completes instead. HT-150-04 and HT-150-05 erase the call's first whole marker and release
  it while the window stays open, and require every later denied try in that call to stay silent;
  pausing on every retry (V03) fails both. HT-150-03 also requires a later terminal-read call in
  the same endpoint to pause on its own (V04 fails).
- **Low 1 is closed.** All seven round 1 survivors (V02, V03, V04, V10, V17, V18, V20) now fail a
  row, as does a new V21. HT-150-01 checks each missing retry companion for `claim`,
  `terminal-read`, and `ignored,claim,terminal-read` against the literal refusal and requires that
  no child Run exists afterwards; it also requires that a selector naming no place changes nothing
  and that the older seams ignore comma lists. HT-150-05 pins `injected endpoint retry pause at
  <place> expired`, and HT-150-04 and HT-150-05 require a live holder's refusal within three
  seconds.
- **Correction to round 1:** its command table gave `t136_lock_contention` as 6 passed. The suite
  has five tests, unchanged from `253a063`, and passed 5 of 5; the 6 was a transcription error.

### Source and fixture review (round 2)

The green diff keeps round 1's shape. In `terminal_snapshot` and the resumed claim the deadline is
still set at the first error of each call; the new `first` flag only decides whether to call
`retry_pause` before the existing deadline check, and the retry, sleep, held answer, and refusal
lines are unchanged. Run validates the after-admission pause's and the retry pause's companions
before `nested_seclude` and admission. `hold_pause` writes the place name and a newline with
`fs::write`, polls the release file without removing it, and expires as `injected endpoint retry
pause at <place> expired`; the old seams and the skill pauses still reach it with exact-value
matching. SVE-005: the lane's `Pause::expect`, `expect_replaced`, and `absent` and the peer's
holding wait all go through `support::await_marker_line`, and `absent` asserts that no read has
succeeded at all. SVE-006: every setting stays on an owned child `Command`; the lane mutates no
process environment, and its only raw file reads are the driver launch log, the ledger, Run and
invocation records, and the Run log, never a marker. The public checks did not change; they still
meet their P4 cells, and all three fail under `l18` (V01).

### Commands on the round 2 bytes

All Cargo commands used `--offline`; test commands also used `--locked`. The set ran once in the
fresh copy with no mutant overlay; the stress runs reused that copy's green lane executable.

| Command/check | Result |
| :--- | :--- |
| Root `cargo build --offline` with a private target directory | Exit 0, no warning; new names absent |
| `cargo test --offline --locked -p ratmac-qa --test t134_safe_nested_invocations -- --exact wtcv_013 wtcv_014 wtcv_015` | 3 passed, 0 failed |
| `cargo test --offline --locked -p ratmac-qa --test t134_safe_nested_invocations` | 7 passed, 0 failed |
| `cargo test --offline --locked -p ratmac-qa --test t126_shared_private_test_support` | 10 passed, 0 failed |
| `cargo test --offline --locked -p ratmac-qa --test t136_lock_contention` | 5 passed, 0 failed |
| `cargo test --offline --locked --manifest-path test-hidden/t-150/Cargo.toml` | 6 passed, 0 failed in 56.63 s; unit targets have no tests |
| Green lane executable alone, four at once twice, six at once | 6 of 6; 24 of 24 twice; 36 of 36; no process or temp entry left |
| `cargo test --offline --locked -p ratmac-qa --test t104_operator_skill` (skill pauses) | 5 passed, 0 failed |
| `cargo fmt --all --check`; `rustfmt --check --edition 2021` on the lane's two Rust files | Exit 0 |
| `cargo clippy --offline --locked --all-targets`: root with and without the fault feature, workspace, lane | Exit 0; no warning in the root package or the lane; the workspace's five are in older `t113` and `t135` lines |
| `python tools/check_links.py` | Exit 0 |

### Wrong implementations tried in round 2

Each was newly built from the round 2 bytes in its own copy with its own root and lane build output,
then ran the three public checks and the six private rows, one variant at a time. V01 to V20 repeat
round 1's damage on the new bytes; V21 is new. No row ended `fixture unavailable`: every credited
failure is a product assertion, or a whole-marker wait for a pause the damage never published.

| Variant / wrong implementation | Public checks that failed | Private rows that failed |
| :--- | :--- | :--- |
| Green control, no overlay | None | None |
| V00: red source, no retry seam | 013, 014, 015 | 01, 02, 03, 04, 05, 06 |
| V01: `l18`, a release window in `RunLock::try_acquire` becomes `Ok(None)` | 013, 014, 015 | 02, 03, 04, 05, 06 |
| V02: pause before starting each call's bound | None | 03 |
| V03: pause on every retry, not the first | None | 04, 05 |
| V04: pause only in the first denied `terminal_snapshot` call | None | 03 |
| V05: new selector exact-only, no comma list | None | 01, 02, 03, 04, 05, 06 |
| V06: append the marker line instead of replacing it | None | 03 |
| V07: Engine removes the retry release file | None | 03 |
| V08: claim pauses once but returns the first open error without retrying | 013, 015 | 02, 03, 04, 05, 06 |
| V09: terminal-read `Err` returns `Ok(None)` | 014 | 02, 03, 04, 05, 06 |
| V10: no pre-admission retry-companion validation | None | 01 |
| V11: any recognized new selector pauses at either place | None | 02 |
| V12: marker records the selector list instead of the place | None | 02, 03, 04, 05, 06 |
| V13: one deadline shared across both places | None | 03 |
| V14: terminal read pauses once but returns the first open error without retrying | 014 | 02, 03, 04, 05, 06 |
| V15: both five-second bounds replaced with one second | 015 | 03, 04, 05 |
| V16: companions validated even when the new selector names no place | None | 01 |
| V17: retry expiry says `timed out`, not `expired` | None | 05 |
| V18: old exact-value seams split comma lists too | None | 01 |
| V19: pause before any denied lock try | 015 | 01, 02, 03, 04, 05, 06 |
| V20: a live claim holder waits out the bound before the held refusal | None | 04, 05 |
| V21: retry companions validated only after admission | None | 01 |

V04 logged four late-window scheduling misses in HT-150-03, then failed it on a terminal-read
whole-marker wait. V01 and V00 fail all three public checks waiting for the missing first-retry
marker. No wrong implementation survives every named check.

## Round 3 (2026-11-18): the repaired lane Engine path

**Accepted; no blocker, medium, or low finding, and no surviving wrong implementation**, at the same
red commit `ba65f9e2e2cfe4331754eb810495b97740bf0b2a`, on `253a063`, plus the same uncommitted green
changes to `src/endpoint.rs` and `src/nested.rs` that round 2 accepted. After round 2 the first full
proof failed in the nested lane sweeps of t108 and t110. Those sweeps start Cargo from the canonical
checkout, spelled with Windows's verbatim marker (`\\?\E:\...`), so this lane's `CARGO_BIN_EXE_rtm`
arrived with the marker. Every declared driver is a `.cmd` file that launches `engine()` through
`cmd.exe`, which cannot run that spelling. Only `test-hidden/t-150/tests/hidden.rs` changed (round 2
`fe5f6f4474251e4c4e4157b5f6ad5ac164b66bb1de5d0f9cdc9bbc4c8cfc2a65`): the private-proof agent's
patch turns the round 2 bytes into the current bytes exactly, and reversed it turns them back
exactly. Every other input in the table is byte-identical to round 2; `src/lock.rs`,
`src/scheduler.rs`, `test/qa/src/support.rs`, `test/qa/tests/t136_lock_contention.rs`, and
`tools/sweep_lanes.py` are unchanged from `253a063`. Round 3 used its own fresh scratch copy,
deleted at its end.

### Source and fixture review (round 3)

The patch has two hunks: 21 lines added and 1 removed.

- `engine()` takes the marker off only when the rest, made absolute, is the rest itself
  (`std::path::absolute(rest) == rest`), the rule of `ratmac_qa::baseline::unmarked`; any other
  spelling comes back unchanged. When Cargo's path carries no marker, as in every plain-path run and
  every run off Windows, it returns what round 2 returned, so round 2's code runs unchanged. The
  unmarked path names the file Cargo built for this lane, so the lane still launches only the
  executable its own build reported (SVE-007). Every `engine()` use gets the one spelling: the
  declared and later-read driver scripts, the Engine command, and the two pin digests.
- HT-150-01 now opens with a fixture precondition: `engine()` is absolute and unmarked, or the row
  fails with a message naming both spellings. It changes no oracle. Where the marker cannot come
  off, the drivers cannot launch and the other five rows fail anyway; this row says why.
- Nothing else moved. `shown()`, `canonical()`, the lock-path and refusal texts, the pause and hold
  settings, the whole-marker waits, the budgets, and the cleanup are byte-identical to round 2. The
  comment line above `engine()` still says it resolves the Engine "exactly as t-149" does; that holds
  for where the path comes from, and the five new comment lines under it state the one difference.

### Commands on the round 3 bytes

All Cargo commands used `--offline`; the plain-path test commands also used `--locked`. The
canonical-path rows called `run_crate` from the copy's own `tools/sweep_lanes.py` for this one
crate, with the copy spelled `\\?\C:\...`: `cargo metadata --no-deps --offline`, then
`cargo test --offline --target-dir <that folder>` with every `CARGO_*` variable removed. Their
dep-info recorded `CARGO_BIN_EXE_rtm=\\?\C:\...\target\debug\rtm.exe`, the spelling the proof met.
Each row ran alone with its own temp folder; after each, no process from the scratch copy was left
and its temp folder was empty. A first plain-path lane attempt never started Cargo (a fault in the
reviewer's run script) and was rerun unchanged.

| Command/check | Result |
| :--- | :--- |
| Patch forward on the round 2 bytes; reversed on the current bytes | `7b07cfb1...` and `fe5f6f44...` exactly |
| Lane, canonical spelling, twice | Pass: 6 of 6, in 72.1 s and 55.0 s; no scheduling miss |
| `cargo test --offline --locked --manifest-path test-hidden/t-150/Cargo.toml` (plain spelling) | 6 passed, 0 failed, in 54.7 s |
| Round 2 lane bytes, canonical spelling | Red, 6 of 6: each nested parent refuses with `rtm: endpoint: the driver exited 3 while run <id> is planned at plan; diagnostic: The system cannot find the path specified.` |
| Only `engine()` reverted, precondition kept, canonical spelling | Red, 6 of 6: HT-150-01 fails its precondition naming `\\?\C:\...\rtm.exe`; the other five fail as above |
| `l18` on the round 3 bytes, canonical spelling | Red: HT-150-02 on the driver's exact launch count, HT-150-03 to HT-150-06 on whole-marker waits; HT-150-01 passes, as in round 2 |
| `cargo test --offline --locked -p ratmac-qa --test t134_safe_nested_invocations -- --exact wtcv_013 wtcv_014 wtcv_015` | 3 passed, 0 failed |
| `cargo test --offline --locked -p ratmac-qa --test t134_safe_nested_invocations` | 7 passed, 0 failed |
| `cargo test --offline --locked -p ratmac-qa --test t126_shared_private_test_support` | 10 passed, 0 failed |
| `cargo test --offline --locked -p ratmac-qa --test t136_lock_contention` | 5 passed, 0 failed |
| `rustfmt --check --edition 2021` on the lane's two Rust files; `cargo clippy --offline --locked --manifest-path test-hidden/t-150/Cargo.toml --all-targets` | Exit 0; no warning |

### Wrong implementations in round 3

Round 3 reran only `l18` of round 2's twenty-one wrong implementations, under the canonical
spelling, where it keeps all five of its round 2 lane catches. In a plain-path run the round 3 lane
runs round 2's code plus one passing precondition, so round 2's matrix stands there. Reverting the
`engine()` fix is caught by every row, and HT-150-01 names the cause. No wrong implementation
survives every named check.

## Known limits for the gap record

- Rounds 2 and 3 carry no finding and no surviving wrong implementation. The matrices and probes are
  this reviewer's observations in scratch copies, not damage receipts; the ticket's own damage pass
  still records its catches in `res-204`.
- The public checks alone catch only V00, V01, V08, V09, V14, V15, and V19; the seam's settled
  interface rules and its pause-budget rules rest on the private lane's six rows.
- Rows that resume through `window_resume` (HT-150-04's peer-dies case, HT-150-05's windowed twins,
  and HT-150-06's current-child case) retry a miss on the same pair after killing its endpoint at
  the retry pause. No run here produced a miss in those rows, so that retry path is unexercised. A
  miss never passes a row, but load well beyond six concurrent lane copies could exhaust the twelve
  attempts and fail a row `fixture unavailable`.
- A plain build still carries the older seams' names (`RATMAC_TEST_ENDPOINT_PAUSE*`,
  `RATMAC_TEST_NESTED_PAUSE*`, and both skill pauses), as at `253a063`; that is outside this ticket.
- Windows only. Other platforms' no-window alternatives were read but not run. No whole-workspace
  proof, lane sweep, turn tool, merge, receipt restamping, or real-checkout Engine run was performed
  by this reviewer. The requested suites and quality commands above are not a full merge proof.
- The public marker deadline is a generous ceiling, not a hard real-time guarantee.
- Under the canonical spelling, round 3 ran only the green lane, the round 2 lane, the `engine()`
  revert, and `l18`; the other twenty wrong implementations were not rerun there.
- The canonical-path rows called the sweep's `run_crate` for this crate alone, not t108 or t110, and
  kept an unmarked temp folder, which the sweep passes through from its caller. A verbatim-marked
  temp folder would also break the drivers' launch-log and later-read paths; that is untested. The
  next full proof is the first run of the repaired lane inside t108 and t110.
- Not reviewed: the untracked turn files `.arca/ticket/t-150-review-record.txt`
  (`0460c81a9e2a098e3c625a7b1cedc2a617be89dbf9aca62f98f8970566212462`) and
  `.arca/ticket/t-150-proof-receipt.txt`
  (`1a828e74ee4b513064cfa791910b2f45903692f44ac4f067538f0b07446e4e13`), both written before the
  lane repair. The review record still lists `t-150/tests/hidden.rs` at the round 2 bytes
  `fe5f6f44...`, so it does not describe the lane reviewed here.

## Reviewed inputs

Changed tracked inputs are relative to `253a063`, including the red commit and uncommitted green.
The four lane files are ignored by Git. SHA-256 describes the exact bytes reviewed in round 3; every
row matched the worktree again after the last round 3 run. Round 1's three differing hashes are
under Round 2, and round 2's one differing hash is under Round 3. This newly created record is
intentionally not included in its own hash table.

| Input | Tracking state | SHA-256 |
| :--- | :--- | :--- |
| `.arca/log.md` | tracked, red commit | `b5ef27f12d6c959501661396ebe40879277733149ed9a302cd9f8a4395ffe892` |
| `.arca/ticket/t-150.md` | tracked, red commit | `811594d1873c07b3d1c5a9b724ab620cc36e1a3fc965e9917225dd966d1fdd90` |
| `.ratmac/evidence/run-074/WTCV-013.toml` | tracked, red commit, new | `f38643b9de73f9fe11cec7d5f1785614cad6dc3ab25ddb6bb9b3592b011505e3` |
| `.ratmac/evidence/run-074/WTCV-014.toml` | tracked, red commit, new | `c503f30e563d5ad84d43c972f1f663da7d88310e6bac9a539668be9271e072c9` |
| `.ratmac/evidence/run-074/WTCV-015.toml` | tracked, red commit, new | `75e4ca84673868f783532e1306fc0333316ae70b3b55ae05f6e7379c25376172` |
| `.ratmac/lanes.toml` | tracked, red commit | `b7e26b7bd9211f7df12cbd01c9ed91f638d86bc61d06eeb78a40b7b54fb091e0` |
| `src/endpoint.rs` | tracked, uncommitted green | `b97bd57a65bf662f37ecb0b7dc7532a06caad0745e4e5244760f01b745421366` |
| `src/nested.rs` | tracked, uncommitted green | `0f7844b91d9b2ed9b4153f53720d8350bde069d33ef6b0bc02d44f5c65d005df` |
| `test-hidden/t-150/Cargo.lock` | lane, ignored by Git, new | `0a02554e8fa7672ab7b15b00d3e8b7eee821e0fb3add47a1125c9ff69d5526e2` |
| `test-hidden/t-150/Cargo.toml` | lane, ignored by Git, new | `59df284bce7482a87efa3d544fca623c1016abc0cff0b2b5d9e918a67117639f` |
| `test-hidden/t-150/src/lib.rs` | lane, ignored by Git, new | `7d965a6a1ba21ea5b35cd0a6eb51eeddca9dc269682857bd23085802a795df83` |
| `test-hidden/t-150/tests/hidden.rs` | lane, ignored by Git, new | `7b07cfb11b723ec12b0f86da02c80bdb8a1ce15c36d203cf8f8247803ef547ac` |
| `test/qa/tests/t126_shared_private_test_support.rs` | tracked, red commit | `ab9f9b31255823b6d0a035a287f2ed25275d8c407371d01749c2cec19aed5ad0` |
| `test/qa/tests/t134_safe_nested_invocations.rs` | tracked, red commit | `970b77a4182e210f3518e50d40e80dcc69b82beb5310fba5b866e954b713b69e` |
