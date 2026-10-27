# Issue test plan
## Verification
| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| A parked barrier stays in its own case (`SVEV-016`) | [SVE-006](spec.md#requirement-records) | Within one test function, the check parks a barrier-armed `rtm hold` at `before-journal-publication` through the same suite helper `wrsv_003_08` uses for every barrier-armed command, with a long timeout (the seam caps it at 30 seconds, `src/blocked.rs` line 53) and no release file, and learns that it is parked from the whole boundary line of its marker. While it stays parked, the check runs `rtm hold` in-process on a second fixture and gets today's success output, exit code 0, and no marker in that fixture; only then does it release the parked hold, which finishes as it does alone. A helper that arms with `std::env::set_var`, as `wrsv_003_08` does today, makes the second hold fail with exit code 1 and `hold refused; hold test barrier expired before before-journal-publication waiting for <the parked hold's release file>`. Candidate: `test/qa/tests/t130_cycle_pause_routes.rs`. |
| The test process carries no barrier (`SVEV-017`) | [SVE-006](spec.md#requirement-records) | While the hold is parked as in the parked-barrier check (`SVEV-016`), the check finds none of the four `RATMAC_TEST_HOLD_BARRIER*` settings in its own process environment, and a probe child started with plain `std::process::Command::new`, which copies the parent environment, reports none either. Today `wrsv_003_08` sets all four in the test process (first at lines 2644-2647), so both probes find them. Candidate: `test/qa/tests/t130_cycle_pause_routes.rs`. |
| Every process-wide arming is case-scoped (`SVEV-018`) | [SVE-006](spec.md#requirement-records) | A source inventory over `test/qa/tests`, `test/qa/src`, and every declared, non-expired private lane (not lane `target/` folders or the frozen before-source corpus under `test-hidden/t-126/fixtures/before/`) finds every place a `RATMAC_TEST_*` setting is set or removed in the test process: `std::env::set_var` or `remove_var` with that name, `StepFault::set`, or a lane wrapper around them such as `env_set` or `fn answered`. Following calls inside the same test binary, it checks that each runs only inside `support::isolated_test`, inside a child of the test binary that runs exactly one named case (`--exact`, as the rule-fix recovery lane's (`t-132`) kill-child role does), or inside a case holding the hook lane (directly or through a wrapper such as `lane_guard`) in a test binary whose every in-process Engine case also holds it. An in-process Engine case calls `cli::run_from` or another Engine library entry in the test process. The definitions of `StepFault` and `HookLane` in `test/qa/src/support.rs` are exempt. Today it fails on `t130_cycle_pause_routes.rs`: 43 such lines in `wrsv_003_08`, while `wrsv_003_01`, `wrsv_003_02`, `wrsv_003_03`, `wrsv_003_04`, and `wrsv_003_07` run Engine commands in-process without the lane. Candidate: `test/qa/tests/t126_shared_private_test_support.rs`. |

These are planned checks, not results. Executable work enters its approved ticket before tests or implementation are authored.
## Goal/Test File Traces
| Goal/Test File | Status | Reverse Issue Refs |
| :--- | :--- | :--- |
| `.arca/goal/index.md` | updated | [Accepted authority](../../schema.md#sve-006--case-scoped-test-hooks) |
| `.arca/goal/ubi-lang.md` | updated | [Accepted authority](../../schema.md#sve-006--case-scoped-test-hooks) |
| `.arca/goal/spec.md` | unaffected | [Accepted authority](../../schema.md#sve-006--case-scoped-test-hooks) |
| `.arca/goal/design.md` | unaffected | [Accepted authority](../../schema.md#sve-006--case-scoped-test-hooks) |
| `.arca/goal/test-list.md` | updated | [Accepted authority](../../schema.md#sve-006--case-scoped-test-hooks) |
## Contributor Authority/Schema Traces
| Authority or Schema Artifact | Status | Integration and Reverse Refs |
| :--- | :--- | :--- |
| `AGENTS.md` | unaffected | No change during filing. |
| `.arca/schema.md` | updated | [Accepted authority](../../schema.md#sve-006--case-scoped-test-hooks) |
| `.arca/wishlist.md` | unaffected | Original wish retained unchanged until its complete desired end is proved. |
| `.ratmac/ratmac.toml` | unaffected | No active Run contract is edited during filing. |
