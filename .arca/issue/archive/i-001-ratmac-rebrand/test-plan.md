# Issue test plan

## Verification

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| `VR-001-active-ssot-vocabulary` | `RAT-001`, `RAT-005` | A constrained repository audit shows `ratmac`/`rtm` in every active SSOT title, definition, command example, requirement, design decision, and test-list check; no unallowlisted active `arca-scheduler`/`schd` reference remains. `ratmac.toml` remains the class filename. |
| `VR-002-package-and-binary-metadata` | `RAT-002`, `RAT-006` | `cargo metadata --no-deps` and the QA manifest report canonical package/library/dependency and binary names (`ratmac`/`rtm`); regenerated `Cargo.lock` contains the canonical package records and no stale active package/bin record. The source path is `src/bin/rtm.rs`; no `src/bin/schd.rs` or duplicate legacy binary is shipped. |
| `VR-003-runtime-cli-surface` | `RAT-003`, `RAT-008` | Real executable smoke runs of `rtm --help`, `rtm start`, `rtm status`, and `rtm step` exercise the same successful/refused behavior as before; help, usage, and error output identify `rtm`, and the selected compatibility policy is observed for `schd`. |
| `VR-004-behavior-regression` | `RAT-004`, `RAT-008` | Full `cargo test` plus the QA and hidden lanes pass with renamed invocations while all existing state, guard, transition, prompt, lock, strict-parser, print-first, and read-only assertions remain green. |
| `VR-005-lock-migration-safety` | `RAT-007` | Focused tests prove the selected `.arca/rtm.lock`/legacy `.arca/schd.lock` policy: concurrent invocations remain arbitrated, a stale/active legacy lock is never silently deleted or bypassed, and state/log/class bytes retain their established ownership and layout. |
| `VR-006-generated-assets` | `RAT-006` | Cargo/project generation completes after metadata changes; checked-in generated files are updated only through their owning tool, ignored `target/` output is absent from the deliverable, and a clean metadata/build check uses the canonical names. |
| `VR-007-quality-gates` | `RAT-004`, `RAT-008` | `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, full tests, and `git diff --check` pass; review confirms no behavior or unrelated files changed. |
| `VR-008-compatibility-record` | `RAT-007` | The integrated goal/design records the command, package/crate, diagnostics, persisted-data, lock-path, and historical-artifact decisions, including any finite deprecation behavior if the clean-cutover default is changed. |

## Goal/Test File Traces

| Goal/Test File | Status | Reverse Issue Refs |
| :--- | :--- | :--- |
| `.arca/current/index.md` | updated | `RAT-001`, `RAT-008` |
| `.arca/current/ubi-lang.md` | updated | `RAT-001`, `RAT-003`, `RAT-005` |
| `.arca/current/spec.md` | updated | `RAT-001`, `RAT-003`, `RAT-007`, `RAT-008` |
| `.arca/current/design.md` | updated | `RAT-001`, `RAT-003`, `RAT-007`, `RAT-008` |
| `.arca/current/test-list.md` | updated | `RAT-001`, `RAT-004`, `RAT-006`, `RAT-007`, `RAT-008` |

## Contributor Authority/Schema Traces

| Authority or Schema Artifact | Status | Integration and Reverse Refs |
| :--- | :--- | :--- |
| `AGENTS.md` | unaffected | It only routes to `.arca/index.md`; verify the route remains valid. |
| `.arca/index.md` | updated | `RAT-005`, `RAT-007`; active working-rule references to the executable use `rtm`, while historical/append-only rules remain intact. |
| `.arca/log.md` | unaffected | `RAT-005`; append-only history is preserved and any retained old names are listed in the audit allowlist. |
| `.arca/ticket/archive/` | unaffected | `RAT-005`; archived ticket provenance is preserved and is not a compatibility surface. |
| `Cargo.toml` | updated | `RAT-002`, `RAT-006`; package/library metadata and binary target become canonical. |
| `test/qa/Cargo.toml` | updated | `RAT-002`, `RAT-004`; QA package, dependency, and binary target become canonical. |
| `Cargo.lock` | updated/generated | `RAT-006`; regenerate with Cargo after manifest changes and verify canonical package records. |
| `src/bin/schd.rs` → `src/bin/rtm.rs` | updated/renamed | `RAT-002`, `RAT-003`; executable entrypoint and diagnostics use `rtm`. |
| `src/cli.rs`, `src/lib.rs`, `src/scheduler.rs`, and related Rust modules | updated | `RAT-002`, `RAT-003`, `RAT-004`; imports, diagnostics, comments, and runtime path constants are canonical without behavior changes. |
| `test/qa/src/`, `test/qa/tests/`, `test/qa/fixtures/` | updated | `RAT-004`, `RAT-008`; executable tests and fixtures use canonical names and preserve behavioral coverage. |
| `.gitignore` and ignored `target/` outputs | unaffected | `RAT-006`; no generated build output becomes a source artifact. |
