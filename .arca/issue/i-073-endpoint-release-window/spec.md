# Issue specification

## Requirement Records

| Requirement ID | Requirement | Disposition | Rationale | Accepted Forward Authority Refs |
| :--- | :--- | :--- | :--- | :--- |
| `WTC-005` | A resumed child Engine endpoint may meet a peer's release window when it claims the child Run's lock, or when it takes its terminal read under that lock. It then waits within the lock module's five-second wait bound and claims or reads once the window closes; a release window is never treated as a held lock. Only a live holder makes the resumed claim refuse, with today's message `rtm: endpoint: the child Run lock for <id> is held; retry once it frees`. A window that outlasts the bound refuses with today's open error, `rtm: endpoint: cannot open lock <path>: <error>`. A named check puts a real release window at both places and fails if the window is treated as held. | accepted | Accepted under Billy's 2026-10-26 wish selection after independent contract review. | [Accepted authority](../../goal/spec.md#integrated-same-verdict-and-windows-timing-requirements) |

## Evidence and boundaries

- The plain single try, `RunLock::try_acquire` ([src/lock.rs](../../../src/lock.rs) line 208), returns an error for a release window (line 228: `cannot open lock <path>: <error>`) and "not acquired" only for a live holder.
- The endpoint ([src/endpoint.rs](../../../src/endpoint.rs)) uses that try in two places. In the terminal read, `fn terminal_snapshot` (line 301, try at line 310), "not acquired" means a transition holds the claim, and the read reports no terminal fact (line 318). In the resumed claim (try at line 731, comment at lines 724-728), "not acquired" refuses `the child Run lock for <id> is held; retry once it frees` (line 739).
- Both places retry an error every 20 milliseconds until the wait bound passes (`WAIT_TIMEOUT`, line 47 of the lock module, five seconds). Then they refuse with that error through `fn refuse` (line 84, prefix `endpoint: `, exit code 1).
- For a resumed child, a terminal read that reports no terminal fact makes the endpoint launch the declared driver again (lines 547-552). So treating a window as held there relaunches work on a finished Run.
- [The lock-wait ticket's review record](../../../.ratmac/evidence/run-059/review.md) records two things. First, the implementer's first version read a window as "not acquired" in the single try, which would have undone the retry added by the safe nested invocation ticket (`t-134`), and was reverted (line 7). Second, the damage probe that made the single try report a window as not acquired (`l18`) passed every check (line 19). [The lock-wait ticket's gap record (`res-110`)](../../residual/archive/res-110.md) (line 53) records the same survivor and names this wish as the carrier of the missing test.
- Existing tests do not reach these places with a window. [The lock contention tests](../../../test/qa/tests/t136_lock_contention.rs) open real windows with the unlink hold, but only for `rtm step` and `rtm start` waiters (`fn observe_window`, line 377, used at lines 554 and 571; the waiters-through-release check (`ENSV-006`), line 499). [The safe nested invocation tests](../../../test/qa/tests/t134_safe_nested_invocations.rs) resume held and interrupted children (`fn webv_022`, line 952; `fn webv_024`, line 2086) with the endpoint pause seam ([src/nested.rs](../../../src/nested.rs) lines 951-958, boundaries `after-admit`, `after-terminal`, `after-receipt`), but never with a window at the child's lock.
- The window can come from the lock module's unlink hold (`RATMAC_TEST_LOCK_HOLD=unlink`, line 1228 of the lock module). A peer pauses after its lock pathname's removal is requested and before its handle closes, so the endpoint's open of that path is denied for as long as the test keeps the peer paused.
- Out of scope: the fresh mint's single try ([src/scheduler.rs](../../../src/scheduler.rs) line 1786), the third caller, which refuses on either answer for a brand-new id. Also out of scope: any change to the endpoint's retry, which already holds.

## Assumptions

- No product behavior changes. If the checks find the retry broken, the repair stays inside the two endpoint places and the single try's contract: a window is an error, and a live holder is "not acquired".
- The checks run on Windows, where release windows exist. Elsewhere they assert the same outcome without a window.
- Test-only seams are compiled only with the `test-fault-injection` feature. Every seam variable is passed to the child process the test starts, never set for the whole test process.

## Dependencies

- Safe nested invocation (`WEB-006`): the parent advances only on the current child's verified terminal receipt. A resumed child neither re-runs its driver nor refuses a passing window.
- The lock split (`ENS-005`): the resumed claim still takes root before Run.
- Other selected issues: the abandon release-window issue (`i-072-abandon-release-window`) shares the meaning of release window. Neither depends on the other.
- The checks wait on pause and lock-hold markers and arm hooks through the environment. So they must not break the whole pause marker issue's rule (`i-067-whole-pause-marker`, requirement `SVE-005`) or the case-scoped hold barrier issue's rule (`i-068-case-scoped-hold-barrier`, requirement `SVE-006`). Neither is a prerequisite.

## Open decisions

How the test learns that the endpoint met the window is left to the ticket (proposed: a test-only pause at the first retry within each call of each place, its boundary value naming the place), and so is which peer command opens it. The disposition is set at integration; the linked forward authority then governs implementation.
