# ratmac goal

## Summary

ratmac (`rtm`) is a thin, deterministic Rust CLI that owns state-machine transitions so agents never run state machines themselves. The Machine is data in a per-project definition file (`ratmac.toml`); the Scheduler is the only engine that steps it. The LLM is a pattern-completer, never a controller: agents read state, never write it, and receive only their Phase Prompt.

## Scope (v1)

- Print-first: `rtm` prints the Phase Prompt to stdout; the Main-Agent or human feeds it into the working session.
- wishwillow's P1–P5 loop is the first Machine Class; the engine holds zero project knowledge.

## Non-goals

- No process spawning or process management in v1; spawn mode, if ever needed, is a future decision, not a dormant code path.
- No agent-journal/log-merge reconciliation across parallel worktrees — harness scope, deferred (see `.arca/log.md`).

## Routes

| Need | File |
| :--- | :--- |
| Terms | [Ubiquitous language](ubi-lang.md) |
| Required behavior | [Specification](spec.md) |
| Decisions and mechanics | [Design](design.md) |
| Verification | [Test list](test-list.md) |

## Integrated issue

Rebrand requirements are integrated from [i-001-ratmac-rebrand](../issue/archive/i-001-ratmac-rebrand/index.md): `RAT-001` through `RAT-008`.

## Integrated external identity

External repository identity requirements are integrated from [i-002-ratmac-external-identity](../issue/archive/i-002-ratmac-external-identity/index.md): `EXT-001` through `EXT-006`. The cutover is a later build operation; this planning pass performs no GitHub, origin, or checkout mutation.

## Integrated Engine trust boundary

Guard-execution, refusal-diagnostic, and goal-freeze requirements are integrated from [i-006-engine-trust-boundary](../issue/archive/i-006-engine-trust-boundary/index.md): `ETB-001` through `ETB-003`.

## Integrated contract-verifying gates

Contract-gate, receipt, ownership, blocked-route, and Run-abandonment requirements are integrated from [i-007-contract-verifying-gates](../issue/archive/i-007-contract-verifying-gates/index.md): `PGE-001` through `PGE-007`.

## Integrated acceptance-oracle integrity

Reviewable-snapshot, archive-aware oracle, and opt-in release lane requirements are integrated from [i-008-honest-acceptance-oracles](../issue/archive/i-008-honest-acceptance-oracles/index.md): `AOI-001` through `AOI-003`.

## Integrated operable Run start

Caller-policy, bootstrap/doctor, and behavioral-evidence requirements are integrated from [i-009-operable-run-start](../issue/archive/i-009-operable-run-start/index.md): `ORS-001` through `ORS-003`. `ORS-001` supersedes the former user-only `rtm start` rule in `R-007`.

## Integrated trial-worktree lifecycle

Trial lifecycle requirements are integrated from [i-010-trial-worktree-lifecycle](../issue/archive/i-010-trial-worktree-lifecycle/index.md): `TWL-001` through `TWL-010`.
