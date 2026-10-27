# Issue test plan

## Verification

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| A release during abandon's lock check passes (`WTCV-010`) | [WTC-004](spec.md#requirement-records) | On Windows, the test-only fault makes the first few presence reads of the root lock and of the addressed Run's lock answer `Access is denied. (os error 5)`, and it records each denial it injects. Under it, `rtm abandon --run <id> --confirm "abandon <id>"` retires the Run with today's success line. An abandon given its project phrase, in a project whose only remnant is a leftover root lock, retires that lock as today. The fault's record shows a denial at each lock path read. Without the retry, both refuse `rtm: abandon refused; abandon cannot inspect lock <path>: Access is denied. (os error 5)`; today's code has no fault, so the record is missing and the check fails. Candidate: `test/qa/tests/t136_lock_contention.rs`. |
| The settled answer decides (`WTCV-011`) | [WTC-004](spec.md#requirement-records) | With the same fault on lock paths that do not exist, the fault's record shows it fired, and abandon's output and exit code are byte-identical to a run without the fault. An unaddressed abandon in a project with no live Run and no lock refuses with today's `rtm: abandon refused; nothing to retire in <project>: no live run and no leftover lock`. An addressed abandon of a live Run whose Run lock is absent retires it as today. Without the retry, both refuse with `abandon cannot inspect lock`; today's code has no fault, so the record is missing. Candidate: `test/qa/tests/t136_lock_contention.rs`. |
| Real failures keep today's refusal (`WTCV-012`) | [WTC-004](spec.md#requirement-records) | When every presence read answers the denial, abandon exits 1 with `rtm: abandon refused; abandon cannot inspect lock <path>: Access is denied. (os error 5)`. It exits no sooner than five seconds after it started, and nothing is retired. When the injected error is another code, such as a sharing violation, the fault's record shows exactly one injected read, and it refuses with the same message shape and that error. On other platforms the read is unchanged. Without the retry, the lasting denial refuses at once; today's code has no fault and retires the Run. Candidate: `test/qa/tests/t136_lock_contention.rs`. |

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
