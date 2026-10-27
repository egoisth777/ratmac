# Issue test plan
## Verification
| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| Empty and partial markers count as unpublished (`SVEV-013`) | [SVE-005](spec.md#requirement-records) | For each staged state, held fixed for the whole call - no file, an empty file, the boundary text without its newline, and the whole line - the shared whole-line wait reports not yet published at a short limit for the first three and returns the line for the last; no step depends on timing. Today the shared whole-line wait does not exist; the waiters it replaces (an empty read accepted at once in `fn await_marker` of the byte-verified restores (`t-124`), checkpoint-only restoration (`t-125`), and edition proof preflight (`t-127`) lanes; a partial line accepted by `fn wait_marker` in `t136_lock_contention.rs`) are named in the inventory check (`SVEV-015`). Candidate: `test/qa/tests/t126_shared_private_test_support.rs`. |
| A planted unfilled marker does not release the waiter (`SVEV-014`) | [SVE-005](spec.md#requirement-records) | For the turn tool's `after-validate` pause and each Engine pause point that writes its marker in place, the test plants an empty file at the marker path (and, in a second run, the boundary text without its newline), then calls the case's waiter with a short limit before starting the paused command: the waiter must report not yet published. The test then starts the command; the waiter acts only after the whole line arrives, and the case's verdict matches a run with nothing planted. Today `fn await_marker` in `t125_checkpoint_only_restoration.rs` (line 90) returns at once on the planted empty file. Candidate: `test/qa/tests/t125_checkpoint_only_restoration.rs` and the review before proofs lane (`t-131`). |
| Every pause-marker wait uses the whole-line rule (`SVEV-015`) | [SVE-005](spec.md#requirement-records) | A source inventory over `test/qa/tests`, `test/qa/src`, and every declared, non-expired private lane finds that every wait on, or read of, a file whose path is passed as a `RATMAC_TEST_*_MARKER` setting (through `.env`, `set_var`, or a lane wrapper such as `hold_env` or `path_env`) goes through the whole-line wait. Checks made after the publishing command has exited, that the marker is absent or still there, are allowed. Lane `target/` folders and the frozen before-source corpus under `test-hidden/t-126/fixtures/before/` are not scanned. Today it fails, listing `fn await_marker` in the byte-verified restores (`t-124`), checkpoint-only restoration (`t-125`), and edition proof preflight (`t-127`) lanes; `fn wait_boundary` and `fn wait_marked` in the successor ownership lane (`t-122`); `fn wait_for_line_starting` in the safe nested invocations lane (`t-134`) and `fn wait_for_file_prefix` in the authorized review transitions lane (`t-129`); the existence-only waits in the lock split (`t-073`), runbook States (`t-080`), Run Record file (`t-081`), one-command suite proof (`t-086`), no work-item (`t-087`), and deterministic trace channel (`t-128`) lanes; line 902 of the lock contention lane (`t-136`); and the waits in nine public suites from `t050_blocked_route.rs` to `t136_lock_contention.rs`. Candidate: `test/qa/tests/t126_shared_private_test_support.rs`. |
| Regression private lane (`HT-142-01`) | [SVE-005](spec.md#requirement-records) | Oracle in ticket t-142's hidden-test table; private suite under `test-hidden/t-142/`. |
| Input/Routing private lane (`HT-142-02`) | [SVE-005](spec.md#requirement-records) | Oracle in ticket t-142's hidden-test table; private suite under `test-hidden/t-142/`. |
| Lifecycle/Model private lane (`HT-142-03`) | [SVE-005](spec.md#requirement-records) | Oracle in ticket t-142's hidden-test table; private suite under `test-hidden/t-142/`. |
| Durability/Recovery private lane (`HT-142-04`) | [SVE-005](spec.md#requirement-records) | Oracle in ticket t-142's hidden-test table; private suite under `test-hidden/t-142/`. |
| Output/Filesystem private lane (`HT-142-05`) | [SVE-005](spec.md#requirement-records) | Oracle in ticket t-142's hidden-test table; private suite under `test-hidden/t-142/`. |
| Cross-Feature private lane (`HT-142-06`) | [SVE-005](spec.md#requirement-records) | Oracle in ticket t-142's hidden-test table; private suite under `test-hidden/t-142/`. |

These are planned checks, not results. Executable work enters its approved ticket before tests or implementation are authored.

The private-lane rows (`HT-`) were added when the tickets were cut on 2026-10-27, so the selected-issue binding (WCP-003) permits the hidden lanes each owning ticket declares; they add no requirement.
## Goal/Test File Traces
| Goal/Test File | Status | Reverse Issue Refs |
| :--- | :--- | :--- |
| `.arca/goal/index.md` | updated | [Accepted authority](../../schema.md#sve-005--whole-pause-marker-line) |
| `.arca/goal/ubi-lang.md` | updated | [Accepted authority](../../schema.md#sve-005--whole-pause-marker-line) |
| `.arca/goal/spec.md` | unaffected | [Accepted authority](../../schema.md#sve-005--whole-pause-marker-line) |
| `.arca/goal/design.md` | unaffected | [Accepted authority](../../schema.md#sve-005--whole-pause-marker-line) |
| `.arca/goal/test-list.md` | updated | [Accepted authority](../../schema.md#sve-005--whole-pause-marker-line) |
## Contributor Authority/Schema Traces
| Authority or Schema Artifact | Status | Integration and Reverse Refs |
| :--- | :--- | :--- |
| `AGENTS.md` | unaffected | No change during filing. |
| `.arca/schema.md` | updated | [Accepted authority](../../schema.md#sve-005--whole-pause-marker-line) |
| `.arca/wishlist.md` | unaffected | Original wish retained unchanged until its complete desired end is proved. |
| `.ratmac/ratmac.toml` | unaffected | No active Run contract is edited during filing. |
