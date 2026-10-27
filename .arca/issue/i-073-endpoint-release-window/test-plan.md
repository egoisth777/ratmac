# Issue test plan

## Verification

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| The resumed claim waits out a release window (`WTCV-013`) | [WTC-005](spec.md#requirement-records) | On Windows, a peer holding the child Run's lock in its unlink hold opens a real release window when the parent's step resumes the child. The endpoint reports reaching its first retry at the claim through the test-only pause. The test then ends the hold and releases the endpoint, which claims the lock. The parent step completes with today's output, and the child Run is admitted once. Under the damage that makes the single try report a release window as not acquired (`l18`), the endpoint refuses `the child Run lock for <id> is held; retry once it frees`, and the check fails. Candidate: `test/qa/tests/t134_safe_nested_invocations.rs`. |
| The terminal read waits out a release window (`WTCV-014`) | [WTC-005](spec.md#requirement-records) | On Windows, a resumed child is already terminal and declares a driver that records each launch. While the endpoint waits at its after-admission pause, the peer's unlink hold opens a real release window, which the endpoint's terminal read then meets. The endpoint reports reaching its first retry at the read, and the test ends the hold. The endpoint delivers the terminal receipt, the parent advances as today, and the driver records no launch. Under the damage that makes the single try report a release window as not acquired (`l18`), the read reports no terminal fact and the driver is launched again, so the check fails. Candidate: `test/qa/tests/t134_safe_nested_invocations.rs`. |
| A live holder and a lasting window keep today's refusals (`WTCV-015`) | [WTC-005](spec.md#requirement-records) | A peer paused while still holding the child Run's lock (the lock module's `release` hold) makes the resumed claim refuse without reaching the first-retry pause. The parent's refusal shows `rtm: endpoint: the child Run lock for <id> is held; retry once it frees`. A release window held past the bound makes the claim refuse with `rtm: endpoint: cannot open lock <path>: Access is denied. (os error 5)`, no sooner than five seconds after the parent step started, and the parent does not advance. Under the damage that makes the single try report a release window as not acquired (`l18`), the lasting window refuses at once with the "is held" text, and the check fails. Candidate: `test/qa/tests/t134_safe_nested_invocations.rs`. |

These are planned checks, not results. Executable work enters its approved ticket before tests or implementation are authored.

## Goal/Test File Traces

| Goal/Test File | Status | Reverse Issue Refs |
| :--- | :--- | :--- |
| `.arca/goal/index.md` | updated | [Accepted authority](../../goal/spec.md#integrated-same-verdict-and-windows-timing-requirements) |
| `.arca/goal/ubi-lang.md` | updated | [Accepted authority](../../goal/spec.md#integrated-same-verdict-and-windows-timing-requirements) |
| `.arca/goal/spec.md` | updated | [Accepted authority](../../goal/spec.md#integrated-same-verdict-and-windows-timing-requirements) |
| `.arca/goal/design.md` | updated | [Accepted authority](../../goal/spec.md#integrated-same-verdict-and-windows-timing-requirements) |
| `.arca/goal/test-list.md` | updated | [Accepted authority](../../goal/spec.md#integrated-same-verdict-and-windows-timing-requirements) |

## Contributor Authority/Schema Traces

| Authority or Schema Artifact | Status | Integration and Reverse Refs |
| :--- | :--- | :--- |
| `AGENTS.md` | unaffected | No change during filing. |
| `.arca/schema.md` | unaffected | No working rule changes; the goal row governs. |
| `.arca/wishlist.md` | unaffected | Original wish retained unchanged until its complete desired end is proved. |
| `.ratmac/ratmac.toml` | unaffected | No active Run contract is edited during filing. |
