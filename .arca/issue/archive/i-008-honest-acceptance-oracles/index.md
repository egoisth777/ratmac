# Reviewable snapshots and honest acceptance oracles

```yaml
issue-id: "i-008-honest-acceptance-oracles"
provenance: "Observed branch recovery evidence, 2026-07-24: the abandoned, uncommitted self-hosted Runbook experiment (formerly branch feat/ratmac-rombook-test at baseline e68bc51) and its independent advisor review. Observed there: the tested candidate consisted largely of untracked packages, QA, and run artifacts that no git diff could review, so the exact tested snapshot was unrecoverable; and the committed external-identity acceptance test failed the whole workspace suite over an authorized completed-issue archive move while also requiring live GitHub identity, branch, and clean-worktree facts inside default test runs. The branch and its untracked artifacts are discarded, so the findings are restated here without links to them."
status: "integrated"
```

## Summary

Two verification foundations failed in the discarded run. First, every green gate was computed against a candidate tree whose load-bearing content — the relocated workspace, the gate helper, new tests, configuration — was untracked, so the reviewed patch omitted what the tests exercised and the exact tested snapshot was never reviewable. Second, the committed acceptance test `test/qa/tests/t038_ext_identity.rs` enumerates every HEAD path under `.arca/` history roots and requires each same path to be unchanged, which misreads an authorized completed-issue archive move as forbidden historical mutation; the same test performs live GitHub, exact-origin, branch, and clean-worktree release checks inside plain `cargo test --workspace`, so ordinary feature-branch work turns the default suite red for reasons unrelated to the change under test. This issue makes acceptance evidence reproducible from reviewable snapshots, makes history-preservation oracles archive-aware under a committed durable policy, and moves environment-coupled release acceptance behind an explicit opt-in. Dispositions in the specification record the author's proposed decision; P1 confirms or revises them at integration.

## Routes

| Need | File |
| :--- | :--- |
| Terms | [Ubiquitous language](ubi-lang.md) |
| Requirements | [Specification](spec.md) |
| Proposed mechanics | [Design](design.md) |
| Verification and integration traces | [Test plan](test-plan.md) |

## Integration

Folded into the goal on 2026-07-24 (P1). Every requirement disposition in [spec.md](spec.md) was confirmed `accepted`.

| Goal artifact | What it now carries |
| :--- | :--- |
| [Goal specification](../../../current/spec.md) | Requirement records `AOI-001`–`AOI-003` |
| [Goal design](../../../current/design.md) | The accepted mechanics for this issue |
| [Goal test list](../../../current/test-list.md) | Checks `AOIV-002`–`AOIV-008` |
| [Goal ubiquitous language](../../../current/ubi-lang.md) | This issue's terms |
| [Goal index](../../../current/index.md) | Reverse link to this issue |
