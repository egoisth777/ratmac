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

Rebrand requirements are integrated from [i-001-ratmac-rebrand](../issue/i-001-ratmac-rebrand/index.md): `RAT-001` through `RAT-008`.

## Integrated external identity

External repository identity requirements are integrated from [i-002-ratmac-external-identity](../issue/i-002-ratmac-external-identity/index.md): `EXT-001` through `EXT-006`. The cutover is a later build operation; this planning pass performs no GitHub, origin, or checkout mutation.
