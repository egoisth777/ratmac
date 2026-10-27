# Issue test plan

## Verification

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| Unreadable item record refuses by name (`WTCV-004`) | [WTC-002](spec.md#requirement-records) | The fixture is one where the validator otherwise passes. The test makes the bound item record exist but unreadable without relying on timing: on Windows it holds the record open with no sharing for the whole run; elsewhere it removes read permission; if the file is still readable (for example, the test runs as root), the case reports that it was skipped and why. `python tools/verify_issue_binding.py check` then exits 1 with empty standard output. Standard error holds exactly one line, starting `verify_issue_binding: cannot read the item record `, naming the record's path and the read error, with no `Traceback`. Today it prints a Python traceback. Candidate: `test/qa/tests/t133_selected_issue_completion.rs`. |
| Unreadable gap record refuses by name and writes nothing (`WTCV-005`) | [WTC-002](spec.md#requirement-records) | The same is done to one of the item's gap records: on Windows a hold with no sharing; elsewhere it removes read permission; if the file is still readable (for example, the test runs as root), the case reports that it was skipped and why. The validator prints one line starting `verify_issue_binding: cannot read the residual record `, exits 1, and leaves standard output empty. A byte snapshot of the fixture, including its Run Records, spawn ledgers, and records, taken before the hold and again after the hold ends and read permission is restored, is the same. Today it prints a Python traceback. Candidate: `test/qa/tests/t133_selected_issue_completion.rs`. |
| The packaged copy refuses the same way (`WTCV-006`) | [WTC-002](spec.md#requirement-records) | `rtm profile plan-build <dir>` exports successfully, and its checker is byte-identical to the tool. Run against both unreadable cases, the checker gives the same exit code and byte-identical standard error as the tool. Today the exported checker prints a Python traceback. Candidate: `test/qa/tests/t135_distributable_plan_build_profile.rs`. |
| Regression private lane (`HT-147-01`) | [WTC-002](spec.md#requirement-records) | Oracle in ticket t-147's hidden-test table; private suite under `test-hidden/t-147/`. |
| Input/Routing private lane (`HT-147-02`) | [WTC-002](spec.md#requirement-records) | Oracle in ticket t-147's hidden-test table; private suite under `test-hidden/t-147/`. |
| Lifecycle/Model private lane (`HT-147-03`) | [WTC-002](spec.md#requirement-records) | Oracle in ticket t-147's hidden-test table; private suite under `test-hidden/t-147/`. |
| Durability/Recovery private lane (`HT-147-04`) | [WTC-002](spec.md#requirement-records) | Oracle in ticket t-147's hidden-test table; private suite under `test-hidden/t-147/`. |
| Output/Filesystem private lane (`HT-147-05`) | [WTC-002](spec.md#requirement-records) | Oracle in ticket t-147's hidden-test table; private suite under `test-hidden/t-147/`. |
| Cross-Feature private lane (`HT-147-06`) | [WTC-002](spec.md#requirement-records) | Oracle in ticket t-147's hidden-test table; private suite under `test-hidden/t-147/`. |

These are planned checks, not results. Executable work enters its approved ticket before tests or implementation are authored.

The private-lane rows (`HT-`) were added when the tickets were cut on 2026-10-27, so the selected-issue binding (WCP-003) permits the hidden lanes each owning ticket declares; they add no requirement.

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
