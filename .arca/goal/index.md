# ratmac goal

## Summary

ratmac (`rtm`) is a thin, deterministic Rust CLI that owns state-machine transitions so agents never run state machines themselves. The Machine is data in a per-project definition file (`ratmac.toml`); the Scheduler is the only engine that steps it. The LLM is a pattern-completer, never a controller: agents read the Run Record, never write it, and receive only their State Prompt.

The Engine runtime is one shared `.ratmac/` Engine root at the primary checkout root: `ratmac.toml`, `runs/`, `mint.toml`, `locks/`, `log.md`, and receipts under `evidence/<run-id>/`. A linked worktree resolves that runtime root while reading its invoking checkout's tracked Machine Class; without Git, the current checkout's `.ratmac/` is the root. Runtime state is Git-ignored, while the Machine Class and run-scoped receipts remain tracked; workflow folders under `.arca/` are reached only through declared `[roots]`.

## Scope (v1)

- Print-first: `rtm` prints the State Prompt to stdout; the Main-Agent or human feeds it into the working session.
- This project's own P1–P5 cycle is the first Machine Class; the engine holds zero project knowledge.

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
| Engine root and runtime | `.ratmac/` at the primary checkout root: `ratmac.toml`, `runs/`, `mint.toml`, `locks/`, `log.md`, and `evidence/<run-id>/`. |

## Integrated issue

Rebrand requirements are integrated from [i-001-ratmac-rebrand](../issue/archive/i-001-ratmac-rebrand/index.md): `RAT-001` through `RAT-008`.

## Integrated external identity

External repository identity requirements are integrated from [i-002-ratmac-external-identity](../issue/archive/i-002-ratmac-external-identity/index.md): `EXT-001` through `EXT-006`. The cutover is a later build operation; this planning pass performs no GitHub, origin, or checkout mutation.

## Integrated Engine trust boundary

Guard-execution, refusal-diagnostic, and goal-freeze requirements are integrated from [i-006-engine-trust-boundary](../issue/archive/i-006-engine-trust-boundary/index.md): `ETB-001` through `ETB-003`.

## Integrated contract-verifying gates

Contract-gate, receipt, ownership, blocked-route, and Run-abandonment requirements are integrated from [i-007-contract-verifying-gates](../issue/archive/i-007-contract-verifying-gates/index.md): `PGE-001` through `PGE-007`. The intake contract treats intake, the Deferred issue buffer, and archive as one issue-id namespace, parses ask dispositions from `spec.md`, and enforces five-file shape, status/location, accepted-goal, and live-link invariants. Every integrated bundle has no deferred ask and at least one accepted-or-duplicate ask; only accepted IDs must exist verbatim as goal rows, so duplicate-only integration is valid.

## Integrated acceptance-oracle integrity

Reviewable-snapshot, archive-aware oracle, and opt-in release lane requirements are integrated from [i-008-honest-acceptance-oracles](../issue/archive/i-008-honest-acceptance-oracles/index.md): `AOI-001` through `AOI-003`. The archive oracle recognizes both complete completed-issue moves into archive and exact complete restoration of an archived deferred bundle to the Deferred issue buffer, while inbound links inside already archived records remain frozen provenance.

## Integrated operable Run start

Caller-policy, bootstrap/doctor, and behavioral-evidence requirements are integrated from [i-009-operable-run-start](../issue/archive/i-009-operable-run-start/index.md): `ORS-001` through `ORS-003`. `ORS-001` supersedes the former user-only `rtm start` rule in `R-007`.

## Integrated trial-worktree lifecycle

Trial lifecycle requirements are integrated from [i-010-trial-worktree-lifecycle](../issue/archive/i-010-trial-worktree-lifecycle/index.md): `TWL-001` through `TWL-010`.

## Integrated Machine Class as first-class data

The Machine Class stops being an implicit shape known only to code. Requirements are integrated from four issues that must land in route order:

- [i-011-runbook-spec](../issue/archive/i-011-runbook-spec/index.md): `RBS-001` through `RBS-005` — the runbook specification at `.arca/runbook-spec.md` is the single written authority for the format, the guard-kind vocabulary, ownership, and the diagnostic-code table.
- [i-012-typed-runbook-parser](../issue/archive/i-012-typed-runbook-parser/index.md): `TRP-001` through `TRP-006` — one typed parse, guards retained, unknown kinds and wrong-for-kind fields refused at parse time, a missing runbook refused by name.
- [i-013-deep-rtm-doctor](../issue/archive/i-013-deep-rtm-doctor/index.md): `DRD-001` through `DRD-007` — the doctor validates through that parser, adds graph and guard lint plus the ownership audit, accepts an arbitrary path, and emits stable codes with `--json` and differentiated exit codes.
- [i-014-agent-authoring-loop](../issue/archive/i-014-agent-authoring-loop/index.md): `AAL-001` through `AAL-004` — `rtm scaffold` plus `.arca/runbook-authoring.md` make write → doctor → repair an agent-runnable loop keyed to those codes.

## Integrated canonical run residency

Run residency requirements are integrated from [i-017-run-residency](../issue/archive/i-017-run-residency/index.md): `FDC-004` through `FDC-006` — Runs reside under the plural `runs` path in one id namespace with `--run <id>` always required, the runbook pin stays hash-only while a flat-layout residue refuses instead of migrating, and run ids are never reused after abandon. The path correction in [i-021-state-file-path-correction](../issue/archive/i-021-state-file-path-correction/index.md) marks the inherited flat State File clauses superseded rather than minting a duplicate requirement. The Ideal-shape property this advances is **One writer, append-only**: an address that is never reissued keeps a finished Run's record from being overwritten by whoever works next. These requirements supersede the v1 single-Run clauses `R-022` and `R-023` plus the flat Run-file clauses of `R-024` and `R-025`.

## Integrated input-routed transitions

Input-routed transition selection is integrated from [i-016-fsm-doctrine-convergence](../issue/archive/i-016-fsm-doctrine-convergence/index.md): `FDC-001`. Branching States declare a closed `inputs` list, ordinary transitions label one value each, static validation proves complete unique coverage, and runtime chooses by current State plus transition input only after readiness guards pass. The carrying Ideal-shape property is **Every boundary machine-checked**: declaration order and agent convention no longer choose a branch.

## Integrated input delivery and durability

Transition-input delivery is integrated from [i-019-input-delivery-durability](../issue/archive/i-019-input-delivery-durability/index.md): `FDC-003`. One strict live verdict record belongs to the addressed Run and current State; the Scheduler consumes it into immutable, collision-free Run evidence before writing successor state, so a consumed input cannot replay. The carrying Ideal-shape property is **One writer, append-only**, with **Every boundary machine-checked** served by validating the record against the current Phase's legal list.

## Integrated Run completion

Run completion is integrated from [i-020-run-completion](../issue/archive/i-020-run-completion/index.md): `FDC-002`. A state with no ordinary outgoing edge is terminal; starting in or arriving at one makes the Engine write `passed` in the same State File replacement, abandonment leaves a durable terminal event naming the Run before active state retires, guard refusal stays non-terminal, and `failed` keeps no write path. The carrying Ideal-shape property is **One writer, append-only**: the terminal fact is Engine-written run state, never an agent claim — and it serves **Self-hosted**, because a composition join and the cycle runbook read that fact.

## Integrated machine composition

Machine composition is integrated from [i-018-machine-composition](../issue/archive/i-018-machine-composition/index.md): `FDC-007` through `FDC-012`. A parent Run spawns declared child classes as ordinary checked motion and finishes on their Engine-written terminal facts: the Scheduler-owned spawn ledger fixes the join's expected set, `respawn` and abandon-with-run-id are phrase-confirmed by run id, every State on a cycle keeps a receipt- or contract-guarded exit, the runbook format carries the class and spawn tables, a spawned child may produce the judgment its parent's branching State consumes, and composition stays one level deep. The carrying Ideal-shape property is **Every boundary machine-checked** — spawn authorization, cycle termination, join evaluation, and the depth cap are Engine-checked boundaries — serving **Self-hosted**, because the cycle runbook delegates ticket turns through exactly this spawn/join surface, with **One writer, append-only** served by the Scheduler-owned ledger.

## Integrated full doctor executable fingerprint

Complete Engine-identity reporting is integrated from [i-023-doctor-full-fingerprint](../issue/archive/i-023-doctor-full-fingerprint/index.md): `DFP-001`. It refines, rather than replaces, `ORS-002` and `DRD-005`: argument-free `rtm doctor` reports the complete SHA-256 of the exact executable it runs while executable selection, write-free diagnosis, trust, state, Runbook findings, and `--json` behavior remain unchanged. The carrying Ideal-shape property is **Every boundary machine-checked**.

## Integrated Engine-namespace split

Engine-root, repository-wide Run identity, split locking, workspace binding, declared workflow roots, residue refusal, and root reporting are integrated from [i-024-engine-namespace-split](../issue/archive/i-024-engine-namespace-split/index.md): `ENS-001` through `ENS-010`. The shared root keeps runtime out of ticket branches while run-scoped tracked receipts let parallel sibling work merge without collision.

## Integrated state vocabulary

The machine position is named State, not Phase, and the Engine's per-Run file is the Run Record. Requirements are integrated from [i-025-state-vocabulary](../issue/archive/i-025-state-vocabulary/index.md): `SVC-001` through `SVC-008`. Three words are separated first — State is the graph position, Run Record is the one file the Engine writes for a Run, Run is the whole live instance, and `status` stays Engine-owned lifecycle — then the rename lands on every live surface: `[states.<name>]` in the runbook, `state` in `.ratmac/runs/<run-id>/run.toml`, and State Prompt in place of Phase Prompt. Pre-cutover runbooks and Run Records refuse and instruct instead of migrating, diagnostic codes keep their identity while their text changes, no behavior moves, and archived history keeps its bytes under an enumerated allowlist. The carrying Ideal-shape property is **Authored, not imitated**: the written schema is the only way the format is meant to be learned, so its words must describe the machine that exists rather than a linear pipeline. **Refusals are branchable** is served too, because the residue gets its own stable code and every pre-existing code survives the rename.