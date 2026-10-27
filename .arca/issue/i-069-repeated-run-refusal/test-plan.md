# Issue test plan

## Verification

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| Each command refuses a repeat (`WTCV-001`) | [WTC-001](spec.md#requirement-records) | With two minted Runs, each of `rtm hold`, `resume`, `recover`, `spawn`, and `respawn` is run with otherwise valid arguments and `--run run-001 --run run-002`, then again with `--run run-001 --run run-001`. Each exits 2, and its standard error starts `rtm: <command>: --run given twice; address exactly one run; runs: ` and goes on to the roster rows. `status`, `step`, `intent`, and `abandon` keep today's bytes as controls. Today none of the five prints this refusal; each goes on with the last value. Candidate: `test/qa/tests/t117_descriptive_run_roster.rs`. |
| A refused repeat changes nothing (`WTCV-002`) | [WTC-001](spec.md#requirement-records) | A byte snapshot of the project tree, including the Run Records, spawn ledgers, `.ratmac/log.md`, and the lock folder, is the same before and after every refused invocation. The wish's `rtm spawn rev --run run-001 --run run-002 --bind ticket=t-7` mints no child and adds no ledger entry. Today that command spawns a child under `run-002`. Candidate: `test/qa/tests/t117_descriptive_run_roster.rs`. |
| The repeat is refused before later checks (`WTCV-003`) | [WTC-001](spec.md#requirement-records) | Each of the five commands also carries a defect that today refuses later: the second value names an unknown Run, the blocker file is missing, the confirmation phrase is wrong, or the spawn name is undeclared. Each still prints the repeated-address refusal, not the later check's refusal. Retired-layout residue still refuses first, byte-for-byte as today. Today the later check's refusal or the action appears instead. Candidate: `test/qa/tests/t120_residue_refusal_precedence.rs`. |
| Regression private lane (`HT-146-01`) | [WTC-001](spec.md#requirement-records) | Oracle in ticket t-146's hidden-test table; private suite under `test-hidden/t-146/`. |
| Input/Routing private lane (`HT-146-02`) | [WTC-001](spec.md#requirement-records) | Oracle in ticket t-146's hidden-test table; private suite under `test-hidden/t-146/`. |
| Lifecycle/Model private lane (`HT-146-03`) | [WTC-001](spec.md#requirement-records) | Oracle in ticket t-146's hidden-test table; private suite under `test-hidden/t-146/`. |
| Durability/Recovery private lane (`HT-146-04`) | [WTC-001](spec.md#requirement-records) | Oracle in ticket t-146's hidden-test table; private suite under `test-hidden/t-146/`. |
| Output/Filesystem private lane (`HT-146-05`) | [WTC-001](spec.md#requirement-records) | Oracle in ticket t-146's hidden-test table; private suite under `test-hidden/t-146/`. |
| Cross-Feature private lane (`HT-146-06`) | [WTC-001](spec.md#requirement-records) | Oracle in ticket t-146's hidden-test table; private suite under `test-hidden/t-146/`. |

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
