# Issue test plan
## Verification
| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| Two requests in one clock tick (`SVEV-010`) | [SVE-004](spec.md#requirement-records) | With the shared helper's clock pinned to one fixed value, two requests for the same label, made from two threads of one test process and released together by a barrier, each receive a different folder that exists, is empty, and was created by that request. The two requests run under a deadline. Dropping the counter from the name makes the check fail at that deadline, because the second request retries the first one's name forever. Creating with `create_dir_all` or deleting a folder to make room is caught by the taken-name check (`SVEV-011`). Today the check cannot be built: the helper's clock cannot be pinned. Candidate: `test/qa/tests/t126_shared_private_test_support.rs`. |
| A taken name moves on (`SVEV-011`) | [SVE-004](spec.md#requirement-records) | Inside an isolated child process, with the clock pinned and the counter's next value known, plant a folder holding a sentinel file at exactly the name the helper will try first. The request returns a different, empty folder, and the planted folder and its file stay byte-identical. A helper that adopts, empties, or deletes the planted folder, or panics on the taken name, fails the check. Today the check cannot be built: the helper's clock cannot be pinned. Candidate: `test/qa/tests/t126_shared_private_test_support.rs`. |
| No test makes its own scratch folder (`SVEV-012`) | [SVE-004](spec.md#requirement-records) | A source inventory over `test/qa/tests`, `test/qa/src`, and every declared, non-expired private lane finds no folder under the system temporary folder made outside the shared helper, apart from the shared-support oracle lane's (`t-126`) own allocator and the freeze build folder (`fn build`, `test/qa/src/baseline.rs` line 175), which the issue giving a private lane one verdict everywhere (`i-065`, `SVE-008`) owns. Lane `target/` folders and the frozen before-source corpus under `test-hidden/t-126/fixtures/before/` are not scanned, and reading the temporary folder's path without making a folder there is allowed. Today it fails, listing 77 clock-named sites in 67 suite files, seven process-and-thread names in six files, four process-id names, the helpers in `test/qa/src/lib.rs`, and `Pair::new` in `test/qa/src/baseline.rs`. Candidate: `test/qa/tests/t126_shared_private_test_support.rs`. |
| Regression private lane (`HT-141-01`) | [SVE-004](spec.md#requirement-records) | Oracle in ticket t-141's hidden-test table; private suite under `test-hidden/t-141/`. |
| Input/Routing private lane (`HT-141-02`) | [SVE-004](spec.md#requirement-records) | Oracle in ticket t-141's hidden-test table; private suite under `test-hidden/t-141/`. |
| Lifecycle/Model private lane (`HT-141-03`) | [SVE-004](spec.md#requirement-records) | Oracle in ticket t-141's hidden-test table; private suite under `test-hidden/t-141/`. |
| Durability/Recovery private lane (`HT-141-04`) | [SVE-004](spec.md#requirement-records) | Oracle in ticket t-141's hidden-test table; private suite under `test-hidden/t-141/`. |
| Output/Filesystem private lane (`HT-141-05`) | [SVE-004](spec.md#requirement-records) | Oracle in ticket t-141's hidden-test table; private suite under `test-hidden/t-141/`. |
| Cross-Feature private lane (`HT-141-06`) | [SVE-004](spec.md#requirement-records) | Oracle in ticket t-141's hidden-test table; private suite under `test-hidden/t-141/`. |

These are planned checks, not results. Executable work enters its approved ticket before tests or implementation are authored.

The private-lane rows (`HT-`) were added when the tickets were cut on 2026-10-27, so the selected-issue binding (WCP-003) permits the hidden lanes each owning ticket declares; they add no requirement.
## Goal/Test File Traces
| Goal/Test File | Status | Reverse Issue Refs |
| :--- | :--- | :--- |
| `.arca/goal/index.md` | updated | [Accepted authority](../../schema.md#sve-004--unique-test-scratch-folders) |
| `.arca/goal/ubi-lang.md` | updated | [Accepted authority](../../schema.md#sve-004--unique-test-scratch-folders) |
| `.arca/goal/spec.md` | unaffected | [Accepted authority](../../schema.md#sve-004--unique-test-scratch-folders) |
| `.arca/goal/design.md` | unaffected | [Accepted authority](../../schema.md#sve-004--unique-test-scratch-folders) |
| `.arca/goal/test-list.md` | updated | [Accepted authority](../../schema.md#sve-004--unique-test-scratch-folders) |
## Contributor Authority/Schema Traces
| Authority or Schema Artifact | Status | Integration and Reverse Refs |
| :--- | :--- | :--- |
| `AGENTS.md` | unaffected | No change during filing. |
| `.arca/schema.md` | updated | [Accepted authority](../../schema.md#sve-004--unique-test-scratch-folders) |
| `.arca/wishlist.md` | unaffected | Original wish retained unchanged until its complete desired end is proved. |
| `.ratmac/ratmac.toml` | unaffected | No active Run contract is edited during filing. |
