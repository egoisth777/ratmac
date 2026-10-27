# Issue test plan

## Verification

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| Sweep crates build apart (`SVEV-019`) | [SVE-003](spec.md#requirement-records) | In a fixture lanes root of two crates below a `.cargo/config.toml` whose `build.target-dir` names one folder outside the fixture, each crate's test writes the path of its own test executable into the fixture. One `tools/sweep_lanes.py sweep` over that fixture reads both crates green, the two recorded paths lie in different folders inside the configured folder, and neither crate's build output appears in the other's folder. Without the setting each path lies in that crate's own `target`; with `--target-dir` both lie in the named folder. Today both paths lie in the configured folder itself. Candidate: `test/qa/tests/t108_lane_sweep.rs`. |
| Shared output untouched (`SVEV-007`) | [SVE-007](spec.md#requirement-records) | In a scratch copy of the repository whose default `target` folder holds stand-ins of different bytes for `rtm`, `rtm-qa`, `edition-cut`, and `snapshot-manifest`, one lane of each kind of Cargo run, run once through `tools/sweep_lanes.py`, gives its standalone verdict and every stand-in keeps its bytes. The lanes are the single-Engine-binary lane (`t-086`), a `today_engine()` caller (`t-116`), the edition-cut and snapshot lanes (`t-127`, `t-113`), an audit-consumer lane (`t-084`), and the lane-sweep lane (`t-108`). The single-Engine-binary lane does the same run alone, also with stand-ins in the folder `CARGO_TARGET_DIR` names. A source inventory over the live lanes finds no `cargo build` or `cargo test` outside the shared locked-folder helper. Today the single-Engine-binary lane reads the `rtm-qa` stand-in and rebuilds `rtm` there, and the callers of `today_engine()` rebuild `rtm-qa` there. Candidate: a new suite in the lane-sweep binary beside `test/qa/tests/t110_post_split_ports.rs`. |
| Forced concurrent runs agree (`SVEV-008`) | [SVE-007](spec.md#requirement-records) | In a scratch copy of the repository whose default `target` is a plain file, each of those six lanes, started twice at once with each run in its own process the way `tools/sweep_lanes.py` runs a crate, gives its standalone verdict in both runs. With the lane folder's lock file held by the check (the lane's own lock, not Cargo's), a started run reports that it waits for that lock and writes nothing in the folder until the check releases it, then gives its standalone verdict. Today every lane that builds there fails to create that folder, and no lane has a lock of its own. Candidate: the same lane-sweep suite. |
| Freeze and sweep folders stay private (`SVEV-009`) | [SVE-008](spec.md#requirement-records) | Run with a fresh system temp folder that holds a plain file named `ratmac-freeze-<commit>`, the rename lane (`t-085`) reads its standalone verdict; one run of the pre-split ports lane (`t-109`) leaves every path and byte under the repository's `target/lanes` as it was, and the pre-split ports suite asserts that its own crate runs leave `target/lanes` as it was. Today the rename lane's freeze build stops with "the freeze Engine builds offline", and the pre-split ports lane's nested sweep and the suite's crate runs build under `target/lanes`. Candidate: a new suite in the lane-sweep binary beside `test/qa/tests/t110_post_split_ports.rs`, and `test/qa/tests/t109_pre_split_ports.rs`. |

These are planned checks, not results. Executable work enters its approved ticket before tests or implementation are authored.

## Goal/Test File Traces

| Goal/Test File | Status | Reverse Issue Refs |
| :--- | :--- | :--- |
| `.arca/goal/index.md` | updated | [SVE-003](../../schema.md#sve-003--sweep-crates-build-apart), [SVE-007](../../schema.md#sve-007--lane-builds-stay-private), [SVE-008](../../schema.md#sve-008--run-private-freeze-and-ports-folders) |
| `.arca/goal/ubi-lang.md` | updated | [SVE-003](../../schema.md#sve-003--sweep-crates-build-apart), [SVE-007](../../schema.md#sve-007--lane-builds-stay-private), [SVE-008](../../schema.md#sve-008--run-private-freeze-and-ports-folders) |
| `.arca/goal/spec.md` | unaffected | [SVE-003](../../schema.md#sve-003--sweep-crates-build-apart), [SVE-007](../../schema.md#sve-007--lane-builds-stay-private), [SVE-008](../../schema.md#sve-008--run-private-freeze-and-ports-folders) |
| `.arca/goal/design.md` | unaffected | [SVE-003](../../schema.md#sve-003--sweep-crates-build-apart), [SVE-007](../../schema.md#sve-007--lane-builds-stay-private), [SVE-008](../../schema.md#sve-008--run-private-freeze-and-ports-folders) |
| `.arca/goal/test-list.md` | updated | [SVE-003](../../schema.md#sve-003--sweep-crates-build-apart), [SVE-007](../../schema.md#sve-007--lane-builds-stay-private), [SVE-008](../../schema.md#sve-008--run-private-freeze-and-ports-folders) |

## Contributor Authority/Schema Traces

| Authority or Schema Artifact | Status | Integration and Reverse Refs |
| :--- | :--- | :--- |
| `AGENTS.md` | unaffected | No change during filing. |
| `.arca/schema.md` | updated | [SVE-003](../../schema.md#sve-003--sweep-crates-build-apart), [SVE-007](../../schema.md#sve-007--lane-builds-stay-private), [SVE-008](../../schema.md#sve-008--run-private-freeze-and-ports-folders) |
| `.arca/wishlist.md` | unaffected | Original wish retained unchanged until its complete desired end is proved. |
| `.ratmac/ratmac.toml` | unaffected | No active Run contract is edited during filing. |
