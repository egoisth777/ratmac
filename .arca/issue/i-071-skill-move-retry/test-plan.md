# Issue test plan

## Verification

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| A brief hold ends and the move completes (`WTCV-007`) | [WTC-003](spec.md#requirement-records) | On Windows, at the test-only pause before the first attempt, the test opens a just-written file inside the partial folder so the first move is denied. It releases the pause and waits for the test-only pause after a denied move. Then it closes the file and releases again. `rtm skill <folder>` exits 0 with today's success line, `Wrote <path>. Its SKILL.md carries the identity stamp of the engine that wrote it.` The folder's files are byte-identical to an unheld write, and no partial folder remains. Without the retry, the move is tried once and the command refuses `rtm: skill: cannot write <path>: Access is denied. (os error 5)`; today's code has no pause, so the check fails waiting for it. Candidate: `test/qa/tests/t104_operator_skill.rs`. |
| A hold that outlasts the bound keeps today's refusal (`WTCV-008`) | [WTC-003](spec.md#requirement-records) | On Windows the test holds the file open from before the first attempt until the command exits. The command exits 2 with exactly `rtm: skill: cannot write <path>: Access is denied. (os error 5)` on standard error, no sooner than five seconds after the test released the pause before the first attempt, and the requested path is absent. Without the retry the refusal comes at once; today's code has no pause, so the check fails waiting for it. Candidate: `test/qa/tests/t104_operator_skill.rs`. |
| A path that appears is never overwritten (`WTCV-009`) | [WTC-003](spec.md#requirement-records) | While the command waits at the pause before the first attempt, the test creates a folder holding one marker file at the requested path, then releases. Without reaching the pause after a denied move, the command exits 2 with today's `rtm: skill: <path> already exists; the skill write never overwrites - choose a new path or delete that one yourself`. The folder and marker file are byte-identical, and no partial folder remains. Without the check, the move is attempted and the command refuses with a `cannot write` error instead; today's code has no pause, so the check fails waiting for it. Candidate: `test/qa/tests/t104_operator_skill.rs`. |

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
