# Issue test plan

## Verification

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| `AOIV-001-issue-shape` | `AOI-001`–`AOI-003` | The pending issue contains exactly the five required populated files with matching identity and provenance, resolved relative routes, and no template markers. |
| `AOIV-002-snapshot-manifest` | `AOI-001` | QA test via `cargo test -p ratmac-qa` (suite names assigned at P4): recording evidence over declared roots in a clean fixture emits a manifest whose per-root digests match an independent re-hash and whose tracking states are all reviewable. |
| `AOIV-003-untracked-refused` | `AOI-001` | Negative: adding one untracked file under a declared evidence root makes evidence recording refuse naming the path, or forces its explicit enumeration as an exception; silent success is a test failure. |
| `AOIV-004-archive-move-pass` | `AOI-002` | In a fixture repository, a complete authorized archive move of a completed five-file issue passes the history-preservation oracle with live links updated. |
| `AOIV-005-mutation-fails` | `AOI-002` | Negative: the same move with one byte of preserved content altered fails naming the file; a partial move leaving one of the five files behind fails naming the gap; an in-place edit of a historical file without a move still fails. |
| `AOIV-006-default-suite-green` | `AOI-003` | On a feature branch with a pending issue folder present and no opt-in configured, `cargo test --workspace` passes and the release acceptance lane visibly reports skipped. |
| `AOIV-007-optin-lane-fails-alone` | `AOI-003` | Negative: with the documented opt-in set in an environment that does not satisfy the lane (for example no authenticated GitHub or a non-release branch), only the release acceptance lane fails, and its diagnostics name the unsatisfied fact; no other suite is affected. |
| `AOIV-008-schema-committed` | `AOI-002` | `.arca/index.md` in the candidate change states the archive authorization and reviewable-snapshot rule, and the change containing it is itself tracked content visible to `git diff`. |

All checks run through the QA harness; the default path requires no commit, push, deployment, network access, or global installation.

## Goal/Test File Traces

| Goal/Test File | Status | Reverse Issue Refs |
| :--- | :--- | :--- |
| `.arca/current/index.md` | updated | Link `i-008-honest-acceptance-oracles` and summarize snapshot and oracle integrity. |
| `.arca/current/ubi-lang.md` | updated | Define reviewable snapshot, snapshot manifest, declared evidence root, authorized archive move, release acceptance lane, and default suite. |
| `.arca/current/spec.md` | updated | Integrate `AOI-001`–`AOI-003` with stable requirement IDs. |
| `.arca/current/design.md` | updated | Record the accepted snapshot-audit, archive-aware oracle, and opt-in lane mechanics. |
| `.arca/current/test-list.md` | updated | Add `AOIV-002`–`AOIV-008`, including the untracked, mutation, partial-move, and opt-in negative cases. |

## Contributor Authority/Schema Traces

| Authority or Schema Artifact | Status | Integration and Reverse Refs |
| :--- | :--- | :--- |
| `AGENTS.md` | unaffected | Caller protocol does not change; evidence and oracle rules live in the contributor schema and QA suites. |
| `.arca/index.md` | updated | Add the completed-issue archive authorization (destination, preservation semantics, link updates, cross-set ID uniqueness) and the reviewable-snapshot evidence rule as durable working rules. |
| `test/qa/tests/t038_ext_identity.rs` | updated | Make the history oracle archive-aware and move the environment-coupled lane behind the documented opt-in with visible skip reporting. |
| `.arca/state.toml`, `.arca/log.md`, `.arca/rtm.lock` | unaffected | Issue creation mutates no Scheduler-owned runtime artifact; later implementation writes them only through `rtm`. |
