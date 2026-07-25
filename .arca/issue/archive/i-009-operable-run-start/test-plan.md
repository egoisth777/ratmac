# Issue test plan

## Verification

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| `ORSV-001-issue-shape` | `ORS-001`–`ORS-003` | The pending issue contains exactly the five required populated files with matching identity and provenance, resolved relative routes, and no template markers. |
| `ORSV-002-policy-surfaces` | `ORS-001` | QA audit via `cargo test -p ratmac-qa` (suite names assigned at P4): CLI help output, root agent protocol, and any present canonical guidance state the human-approved Main-Agent start policy with no active user-only or never-agent-start wording. |
| `ORSV-003-audit-sensitive` | `ORS-001` | Negative: seeding the retired user-only sentence into a fixture copy of an audited surface makes the stale-policy audit fail naming the surface. |
| `ORSV-004-engine-boundary` | `ORS-001` | Code and schema checks find no caller identity, sign-off token, approval artifact, or new Runbook or State File field; existing start, status, step, and duplicate-start refusal regressions pass unchanged. |
| `ORSV-005-bootstrap-resolves` | `ORS-002` | From a clean project root the documented bootstrap command reports the resolved Stable Engine path and content hash, mutating no global configuration or PATH; the command and its output are captured as test evidence. |
| `ORSV-006-pin-mismatch-refused` | `ORS-002` | Negative: with a pin record present and a deliberately altered binary at the resolved location, the bootstrap refuses naming observed and expected identity instead of reporting success. |
| `ORSV-007-doctor-actionable` | `ORS-002` | With no active Run, doctor output distinguishes `.arca/ratmac.toml` from `.arca/state.toml` by role and names the next legitimate action; with an active fixture Run it reports the phase; before-and-after filesystem snapshots prove zero writes in both cases. |
| `ORSV-008-behavioral-roles` | `ORS-003` | Role-scenario transcripts pass: human start invoked, signed-off Main-Agent start invoked exactly once, unsigned Main-Agent and Subagent transcripts contain zero `rtm` invocations; each check's recorded evidence kind is behavioral. |
| `ORSV-009-violation-fails` | `ORS-003` | Negative: a violating transcript in which an unsigned Main-Agent invokes start fails the behavioral check; a wording-only check run against the same scenario is recorded as guidance-consistency and does not satisfy the behavioral requirement. |

All checks run through the QA harness in fixture projects or recorded scenarios; none require commit, push, deployment, network access, or global installation.

## Goal/Test File Traces

| Goal/Test File | Status | Reverse Issue Refs |
| :--- | :--- | :--- |
| `.arca/current/index.md` | updated | Link `i-009-operable-run-start` and summarize the operable caller loop. |
| `.arca/current/ubi-lang.md` | updated | Define Run-start sign-off, project-local bootstrap, doctor report, behavioral evidence, and guidance-consistency evidence. |
| `.arca/current/spec.md` | updated | Integrate `ORS-001`–`ORS-003` with stable requirement IDs, superseding the stale user-only start statements. |
| `.arca/current/design.md` | updated | Record the accepted policy-surface, bootstrap, doctor, and behavioral-harness mechanics. |
| `.arca/current/test-list.md` | updated | Add `ORSV-002`–`ORSV-009`, including the stale-wording, pin-mismatch, and violating-transcript negative cases. |

## Contributor Authority/Schema Traces

| Authority or Schema Artifact | Status | Integration and Reverse Refs |
| :--- | :--- | :--- |
| `AGENTS.md` | updated | State the human-approved Main-Agent start policy, the Subagent read-only rule, and the documented bootstrap entry point. |
| `src/cli.rs` (current checkout layout) | updated | Replace the stale user-only `rtm start` help text and host the read-only doctor behavior. |
| `.arca/index.md` | updated | Record the bootstrap and doctor as the documented orientation defaults for a fresh session. |
| `.arca/state.toml`, `.arca/log.md`, `.arca/rtm.lock` | unaffected | Issue creation mutates no Scheduler-owned runtime artifact; the doctor is read-only and later implementation writes them only through `rtm`. |
