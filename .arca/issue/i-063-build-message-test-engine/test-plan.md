# Issue test plan

## Verification

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| Redirected build runs (`SVEV-001`) | [SVE-001](spec.md#requirement-records) | In child test processes whose build output is moved to a fresh folder - once each by `CARGO_TARGET_DIR` and `CARGO_BUILD_TARGET_DIR`; once by a `build.target-dir` setting in a `.cargo/config.toml` written into a scratch copy of the repository, with the redirect variables cleared and the checkout's own `.cargo/config.toml` never written; and once by `CARGO_BUILD_TARGET` set to the host target `rustc -vV` reports, with `CARGO_TARGET_DIR` naming a fresh folder - the shared function, given that build's root (the scratch copy for the configuration case, the repository otherwise), returns exactly the `executable` Cargo reported for `rtm-qa`, the same path a separate `cargo build --message-format=json` with the same settings reports; under the host-target redirect, with `TMP` and `TEMP` naming a fresh folder, the freeze build also returns what Cargo reported. Today the baseline support has no function that takes a build root, `today_engine()` takes the repository's `target/debug/rtm-qa` or stops with "no Engine at", and the freeze build stops with "no freeze Engine at". Candidate: a new public suite beside `test/qa/tests/t126_shared_private_test_support.rs`. |
| Older binary never runs; no report refuses (`SVEV-002`) | [SVE-001](spec.md#requirement-records) | In a scratch copy of the repository whose `target/debug` holds a stand-in `rtm-qa` of different bytes, with the output moved by `CARGO_TARGET_DIR` to a fresh folder, the shared function, given the scratch copy as its build root, returns exactly the `executable` Cargo reported for `rtm-qa`, the same path a separate `cargo build --message-format=json` with the same settings reports; the stand-in never runs and keeps its bytes; asked for a binary the build did not report, the function refuses naming that binary and the build command, and runs nothing; with a source file made not to compile, the refusal carries the compiler's error text. Today the baseline support has no function that takes a build root, and `today_engine()` takes the fixed-folder path with no such refusal. Candidate: a new public suite beside `test/qa/tests/t126_shared_private_test_support.rs`. |
| One shared answer (`SVEV-003`) | [SVE-001](spec.md#requirement-records) | A source inventory over the test crate and the live private lanes, skipping the frozen before-source corpus under `test-hidden/t-126/fixtures/before/`, finds `--message-format` and `compiler-artifact` only in `test/qa/src/baseline.rs`, no lane that refuses because `CARGO_TARGET_DIR` is set, and no `debug` or `release` folder joined to `rtm`, `rtm-qa`, `edition-cut`, or `snapshot-manifest` except at the out-of-scope sites listed under Evidence and boundaries, which the check names file by file. Today it finds thirteen lane readers, the refusing lane (`t-116`), and the fixed joins in `today_engine()`, the freeze build, both building tests of the single-Engine-binary suite, `fn build_edition_cut`, and the single-Engine-binary, snapshot, edition-cut, and TOML-reading lanes (`t-086`, `t-113`, `t-127`, `t-133`). Candidate: `test/qa/tests/t126_shared_private_test_support.rs`. |

These are planned checks, not results. Executable work enters its approved ticket before tests or implementation are authored.

## Goal/Test File Traces

| Goal/Test File | Status | Reverse Issue Refs |
| :--- | :--- | :--- |
| `.arca/goal/index.md` | updated | [Accepted authority](../../schema.md#sve-001--test-engine-from-build-messages) |
| `.arca/goal/ubi-lang.md` | updated | [Accepted authority](../../schema.md#sve-001--test-engine-from-build-messages) |
| `.arca/goal/spec.md` | unaffected | [Accepted authority](../../schema.md#sve-001--test-engine-from-build-messages) |
| `.arca/goal/design.md` | unaffected | [Accepted authority](../../schema.md#sve-001--test-engine-from-build-messages) |
| `.arca/goal/test-list.md` | updated | [Accepted authority](../../schema.md#sve-001--test-engine-from-build-messages) |

## Contributor Authority/Schema Traces

| Authority or Schema Artifact | Status | Integration and Reverse Refs |
| :--- | :--- | :--- |
| `AGENTS.md` | unaffected | No change during filing. |
| `.arca/schema.md` | updated | [Accepted authority](../../schema.md#sve-001--test-engine-from-build-messages) |
| `.arca/wishlist.md` | unaffected | Original wish retained unchanged until its complete desired end is proved. |
| `.ratmac/ratmac.toml` | unaffected | No active Run contract is edited during filing. |
