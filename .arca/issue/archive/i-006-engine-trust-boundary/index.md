# Trustworthy Runbook gate execution and goal freeze

```yaml
issue-id: "i-006-engine-trust-boundary"
provenance: "Observed branch recovery evidence, 2026-07-24: the abandoned, uncommitted self-hosted Runbook experiment (formerly branch feat/ratmac-rombook-test at baseline e68bc51) and its independent advisor review. That run routed every phase transition through a command guard that rebuilt and ran mutable candidate QA code, printed refusals stripped of the gate's diagnostics, and cited a pre-integration goal hash in every residual. The branch and its untracked artifacts are discarded, so the findings are restated here without links to them."
status: "integrated"
```

## Summary

The discarded self-hosted Runbook run proved that the Engine's determinism guarantees stopped at the `rtm` binary itself. Every Exit and Route Guard invoked `cargo run` against the mutable candidate worktree, so editing the gate helper's source could flip any transition while the pinned Stable Engine stayed byte-identical. When a command guard refused, the Engine had wired the child's stdout and stderr to null, so a gate that printed the exact blocking ticket and residual surfaced only `exit 1`, breaking the documented fix-the-named-artifact repair loop. And the goal revision was hashed once at `rtm start`, before P1 integration rewrote `.arca/current/`, so every residual cited a freeze that did not describe the requirements being classified. This issue extends the Engine trust boundary to cover what guards execute, what refusals report, and when the goal is frozen. Dispositions in the specification record the author's proposed decision; P1 confirms or revises them at integration.

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
| [Goal specification](../../../current/spec.md) | Requirement records `ETB-001`–`ETB-003` |
| [Goal design](../../../current/design.md) | The accepted mechanics for this issue |
| [Goal test list](../../../current/test-list.md) | Checks `ETBV-002`–`ETBV-008` |
| [Goal ubiquitous language](../../../current/ubi-lang.md) | This issue's terms |
| [Goal index](../../../current/index.md) | Reverse link to this issue |
