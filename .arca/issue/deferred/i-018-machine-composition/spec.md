# Issue specification

Each ask below states a settled position from the closed adversarial-review ledger of the
doctrine-convergence evidence seed ([test-plan.md](../../archive/i-016-fsm-doctrine-convergence/test-plan.md)),
written as a requirement candidate for P1 fold-in. Dispositions record the author's proposed decision;
P1 confirms or revises them at integration. Every rationale names its decision provenance: a **human
ruling** (not revocable by an agent) or an **adopted default** (revocable; batch human sign-off,
2026-07-29), both recorded in the seed's decision records
([design.md](../../archive/i-016-fsm-doctrine-convergence/design.md)).

Moved from the i-016 seed at the 2026-07-29 split.

`FDC` expands to **FSM Doctrine Convergence** - the requirement-ID prefix shared across the split,
defined in [ubi-lang.md](ubi-lang.md); IDs stay stable and are never renumbered.

## Requirement Records

| Requirement ID | Requirement | Disposition | Rationale | Accepted Forward Authority Refs |
| :--- | :--- | :--- | :--- | :--- |
| `FDC-007` | Authorization splits by motion kind: `spawn` is ordinary motion with no confirmation phrase; `respawn` and abandon-with-run-id require confirmation phrases naming the run id. | deferred | Composition still awaits durable Run completion after routing and delivery. | — (deferred 2026-07-30) |
| `FDC-008` | Every Phase on a cycle carries at least one outgoing edge guarded by receipt- or contract-class guards. | deferred | The cycle and join contract belongs to the later composition stratum. | — (deferred 2026-07-30) |
| `FDC-009` | The runbook format carries class and spawn tables, with `blocked-route` as the canonical spelling. | deferred | Format expansion follows the routing/delivery and completion contracts it consumes. | — (deferred 2026-07-30) |
| `FDC-010` | Judge independence lands child-as-reviewer first; the witnessed verdict verb remains deferred. | deferred | Child judgment requires spawn/join mechanics; signer identity remains outside the Engine. | — (deferred 2026-07-30) |
| `FDC-011` | The spawn-ledger contract, at the per-run path `FDC-004` reserves under the parent Run's directory: Scheduler-owned, append/annotate-only — agents never write it. `rtm spawn` appends one entry carrying the child run id, child class, binding values, the git revision at spawn, and the child workspace path when one is created. Human-confirmed abandon flips only that entry's abandoned mark; human-confirmed respawn appends the successor entry recording the superseded id (`FDC-006`'s remainder). The ledger fixes the join's expected set: an entry whose child Run is missing on disk refuses loudly, never silently shrinks the set. | pending | Home ruled 2026-08-03 (Billy): this spec carries the contract, extending the 2026-07-29 scope settlement (accepted review fix) that reserved the path in run residency and named this issue the contract's home. Contents condensed from the research's spawn-ledger section ([05-invocation-join.md](../../../research/re-ratmac-FSM/05-invocation-join.md), "The spawn ledger"). | — (pending) |
| `FDC-012` | Composition is capped at one level: a spawned child Run may not itself spawn, and the Engine refuses a spawn addressed to a Run recorded as a child in any spawn ledger, naming the cap. | pending | Individual human ruling, 2026-08-03 (Billy), resolving this issue's recursion-depth fork: capped until need shows; the cap is checkable at the spawn boundary and lifting it later is additive. | — (pending) |

## Ruled fork — recursion depth (was: open question)

- **Recursion depth** — whether a child may spawn grandchildren in the first increment. **Ruled
  2026-08-03 (Billy, individual human ruling): composition is capped at one level until need shows.**
  The ruling enters the asks as `FDC-012`; lifting the cap is additive and needs a new ruling or a new
  issue, never an agent choice. The fork stood unruled from the 2026-07-29 split until this ruling.

## Acceptance criteria

- Every requirement above names its provenance. `FDC-007`–`FDC-010` trace to their decision records in the
  seed's [design.md](../../archive/i-016-fsm-doctrine-convergence/design.md): human ruling or adopted
  default (revocable). `FDC-011` and `FDC-012` trace to the individual human rulings of 2026-08-03 (Billy),
  recorded in this bundle ([index.md](index.md), Disposition log); `FDC-011` additionally extends the
  2026-07-29 scope settlement (accepted review fix) that named this issue the ledger-contract home.
- **Billy's 2026-07-30 cut** created input-routed transitions, input delivery and durability, and
  Run completion as separate pending concerns. **Assumed dependency forecast, revocable at planning
  step 1:** integrated Run residency supplies addresses; input-routed transitions
  ([i-016-fsm-doctrine-convergence](../../archive/i-016-fsm-doctrine-convergence/index.md)) supply legal inputs
  and edge selection; input delivery
  ([i-019-input-delivery-durability](../../archive/i-019-input-delivery-durability/index.md)) supplies
  replay-safe judgment handoff; Run completion
  ([i-020-run-completion](../../archive/i-020-run-completion/index.md)) supplies durable terminal facts.
  Machine composition follows those four contracts.
- The recursion-depth fork stayed recorded and unresolved until a ruling answered it; ruled 2026-08-03
  (Billy) and recorded as `FDC-012` — the record and its provenance survive integration.
