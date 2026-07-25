# Issue test plan

## Verification

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| `ETBV-001-issue-shape` | `ETB-001`–`ETB-003` | The pending issue contains exactly the five required populated files with matching identity and provenance, resolved relative routes, and no template markers. |
| `ETBV-002-pin-recorded` | `ETB-001` | QA test via `cargo test -p ratmac-qa` (suite name assigned at P4): an isolated fixture Run with a project-derived command guard records the gate artifact's resolved path and content hash in Run evidence no later than first guard use, alongside the Stable Engine pin. |
| `ETBV-003-pin-tamper-refused` | `ETB-001` | Negative: after the pin, overwriting the gate artifact's bytes makes the next step request refuse with observed-versus-expected identity and no state or history mutation; restoring the exact bytes lets the identical request proceed. |
| `ETBV-004-no-eval-time-build` | `ETB-001` | Negative: a fixture Runbook whose guard command would rebuild workspace sources at evaluation time (a `cargo run`-style invocation) is rejected at validation or pin time with a named reason; additionally, timestamps and build outputs under the fixture prove no compilation occurred during a legitimate guard evaluation. |
| `ETBV-005-diagnostic-captured` | `ETB-002` | A failing guard program that prints `blocking artifact: <path>` to stderr produces a refusal containing that exact text plus program and exit facts, observable through the real `rtm` CLI in a fixture project. |
| `ETBV-006-diagnostic-bounded` | `ETB-002` | Negative: a failing guard program emitting output far beyond the documented bound produces a refusal no larger than the bound plus fixed framing, containing the truncation marker; a silent failing program produces the documented no-diagnostic wording. |
| `ETBV-007-post-integration-freeze` | `ETB-003` | In a fixture Run whose intake phase rewrites `.arca/current/`, recorded evidence distinguishes the start baseline revision from the frozen goal revision, the frozen value equals the post-integration content hash, and gap-analysis output cites the frozen value. |
| `ETBV-008-drift-refused` | `ETB-003` | Negative: after the freeze, editing a `.arca/current/` file makes the next transition request refuse naming goal drift with frozen and observed revisions; reverting the edit clears the refusal; Scheduler-owned files are byte-identical across the refusal. |

All checks run through the QA harness (`cargo test -p ratmac-qa` and the full `cargo test --workspace`) in isolated fixture projects; none require commit, push, deployment, network access, or global installation.

## Goal/Test File Traces

| Goal/Test File | Status | Reverse Issue Refs |
| :--- | :--- | :--- |
| `.arca/current/index.md` | updated | Link `i-006-engine-trust-boundary` and summarize the extended Engine trust boundary. |
| `.arca/current/ubi-lang.md` | updated | Define Stable Engine pin, pinned gate artifact, refusal diagnostic, start baseline revision, frozen goal revision, and goal drift. |
| `.arca/current/spec.md` | updated | Integrate `ETB-001`–`ETB-003` with stable requirement IDs. |
| `.arca/current/design.md` | updated | Record the accepted pinning, diagnostic-capture, and freeze-boundary mechanics. |
| `.arca/current/test-list.md` | updated | Add `ETBV-002`–`ETBV-008`, including the negative tamper, oversize-diagnostic, and drift cases. |

## Contributor Authority/Schema Traces

| Authority or Schema Artifact | Status | Integration and Reverse Refs |
| :--- | :--- | :--- |
| `AGENTS.md` | unaffected | The refusal-repair loop it documents becomes truthful; its text does not change for this issue. |
| `.arca/index.md` | updated | Add durable safety invariants: command guards execute only pinned or explicitly exempt programs, refusals carry bounded diagnostics, and the frozen goal revision is post-integration and drift-checked. |
| `src/scheduler.rs` (current checkout layout) | updated | Implementation surface for pin verification, bounded stderr capture, and freeze-boundary revision handling. |
| `.arca/state.toml`, `.arca/log.md`, `.arca/rtm.lock` | unaffected | Issue creation mutates no Scheduler-owned runtime artifact; later implementation writes them only through `rtm`. |
