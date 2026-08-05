# Trial log: trial-002-doctor-full-fingerprint

## Identity

- trial: trial-002-doctor-full-fingerprint
- base commit: 02cf1a9cfc6b3829b976ff833632c0a801ee1938
- terminal commit: 329aeafd7c3ecf777c615f9719b1e0bd2e604f19

## Hypothesis

`rtm doctor` can make its Engine identity unambiguous by printing the complete 64-character SHA-256 of the exact executable it runs instead of a 16-character prefix, while remaining read-only.

## Procedure

1. Main started `run-001`. Its initial step refused because `artifacts/release.txt` was missing, and repeated status remained `build/planned`.
2. Two delegated workers claimed a red test and a source edit, but inspection found neither change. The earlier 242-pass public run was therefore not accepted as evidence for this feature.
3. The actual `doctor_reports_full_hash_of_the_test_built_executable` regression was written and run red; it observed a 16-character Engine hash where 64 characters were required.
4. The implementation changed only `&engine_hash[..16]` to `engine_hash` in the doctor report. The focused regression then passed.
5. Main advanced the run from build to build-review. An independent reviewer approved after independently rerunning focused read-only tests and comparing the hash; Main then advanced the run to build-done with status passed.

## Commands and tests

- The focused `rtm doctor` regression, `doctor_reports_full_hash_of_the_test_built_executable` in `test/qa/tests/t045_bootstrap_doctor.rs`, first failed with 0 passed and 1 failed because the reported hash length was 16 rather than 64; it passed after the one-line source change.
- The complete `t045_bootstrap_doctor` target passed: 8 passed, 0 failed, 0 ignored.
- Public workspace verification passed: 243 passed, 0 failed, with 1 opt-in release test ignored.
- The formatting check and warning-denying Clippy verification passed.

## Observations

- `environment_report` now emits the entire SHA-256, and the focused test checks a 64-character lowercase hexadecimal value, equality with the exact test-built `rtm` executable digest, and unchanged filesystem snapshot.
- Run evidence records the Stable Engine digest `947fb4b2e9ad732ea7f70a0a4056b39825d00e3f61be0fcdbe8f481ba73c831b`; CertUtil matched the full recorded baseline hash. The release artifact records a full candidate digest and green focused, public-suite, and Clippy results.
- The final run state is `build-done` with status `passed`; the transition log records build to build-review and build-review to build-done.
- The demo runbook guards only the agent-writable `artifacts/release.txt` directory and `ready: true` content, plus an exempt `rustc --version` check. It has no issue intake, test receipt, completion gate, or P1-P5 workflow, so its guard outcome is a workflow-performance observation rather than proof of the fingerprint feature.

## Verdict

adopt: the full doctor fingerprint is correct in this trial; the expected-red regression, focused fix, independent review, and public verification support normal development.

## Recommendations

- Enter normal development for the full doctor fingerprint and carry the validated behavior through the ordinary main-first development path.
- Do not merge, cherry-pick, or otherwise carry this trial implementation into `main` or the experiment base. Trial policy preserves only the durable trial log, so the normal development change must be made and verified independently.
- Do not use the current demo runbook guard result as feature acceptance evidence; it is unrelated to the feature and depends on an agent-writable release artifact.

## Artifacts and diffs

- `src/cli.rs`, `environment_report`: the only product-source change replaces the truncated `&engine_hash[..16]` value with `engine_hash` in the Engine report.
- `test/qa/tests/t045_bootstrap_doctor.rs`: `doctor_reports_full_hash_of_the_test_built_executable` verifies the exact executable digest, 64-character length, and read-only snapshot behavior.
- `.arca/runs/run-001/evidence.toml`, `.arca/runs/run-001/state.toml`, and `.arca/log.md` record the pinned Engine, final passed state, and the two transitions. `artifacts/release.txt` records the release readiness and verification summary.
- The trial began at `02cf1a9cfc6b3829b976ff833632c0a801ee1938` and ends at `329aeafd7c3ecf777c615f9719b1e0bd2e604f19`; the template is `.arca/tpl/trial-log.md` and the lifecycle requirements are in `.arca/goal/spec.md` under the trial-worktree section.
