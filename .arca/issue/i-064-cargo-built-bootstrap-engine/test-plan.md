# Issue test plan

## Verification

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| Redirected build is found (`SVEV-004`) | [SVE-002](spec.md#requirement-records) | In a fixture copy like the bootstrap doctor suite's, with the output moved once by `CARGO_TARGET_DIR` to a folder outside the fixture and once each by a `build.target-dir` setting in the fixture's `.cargo/config.toml` naming a folder outside the fixture and a folder inside it other than `target`, `pwsh -NoProfile -File tools/rtm.ps1` exits 0, its `Engine:` line names the executable Cargo reported in that folder, `sha256:` is that file's hash, and a pin recorded for it reads `Pin: matches`. Then the fixture's source for the `rtm` binary is changed so it no longer compiles: with the folder inside the fixture, a second run still exits 0 with the same `Engine:` and `sha256:` lines; with a folder outside, the second run builds and refuses with "the project-local build did not produce an Engine". Today every first run refuses with "the build succeeded but no Engine binary is at the expected path". Candidate: `test/qa/tests/t045_bootstrap_doctor.rs`. |
| No fixed-folder stand-in (`SVEV-005`) | [SVE-002](spec.md#requirement-records) | With the output redirected by `CARGO_TARGET_DIR` to an empty folder outside the fixture and stand-in binaries of different bytes planted at `target/release` and `target/debug` under the fixture, the bootstrap builds and reports the fresh binary's path and hash, never a stand-in's, and the stand-ins stay byte-identical; pinning a stand-in's hash refuses with "the resolved Engine is not the pinned Engine". For `-Channel stable` with `CARGO_TARGET_DIR` inherited, the bootstrap builds into the build checkout's own `target`, reports Cargo's executable there, and leaves the redirect folder empty; with a relative `CARGO_BUILD_BUILD_DIR` inherited as well, a second stable run passes the clean-tree check. Today it reports the `target/release` stand-in without building, and the stable build writes the redirect folder and then refuses with "the build succeeded but no Engine binary is at the expected path". Candidate: `test/qa/tests/t045_bootstrap_doctor.rs` and `test/qa/tests/t102_stable_bootstrap.rs`. |
| Nothing reported refuses by name (`SVEV-006`) | [SVE-002](spec.md#requirement-records) | With a stand-in `cargo` first on the child's `PATH` that succeeds but reports no `rtm` executable and resolves an empty output folder, and a stand-in binary at `target/debug`, the bootstrap exits 1 with a `bootstrap refused:` line naming what it searched and the repair, prints no `Engine:` line, and never names the stand-in. Today it reports the `target/debug` stand-in and exits 0. Candidate: `test/qa/tests/t045_bootstrap_doctor.rs`. |

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
