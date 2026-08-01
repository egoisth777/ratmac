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

## Open question (not an ask)

- **Recursion depth** — may a child spawn grandchildren in the first increment, or is composition
  capped at one level until need shows? Unruled: no human ruling and no adopted default covers it.
  This issue records the fork and does not resolve it; the answer lands as a ruling, a goal
  requirement, or a new issue — never as an agent choice.

## Acceptance criteria

- Every requirement above traces to its decision record in the seed's
  [design.md](../../archive/i-016-fsm-doctrine-convergence/design.md) and names its provenance: human ruling or
  adopted default (revocable).
- **Billy's 2026-07-30 cut** created input-routed transitions, input delivery and durability, and
  Run completion as separate pending concerns. **Assumed dependency forecast, revocable at planning
  step 1:** integrated Run residency supplies addresses; input-routed transitions
  ([i-016-fsm-doctrine-convergence](../../archive/i-016-fsm-doctrine-convergence/index.md)) supply legal inputs
  and edge selection; input delivery
  ([i-019-input-delivery-durability](../../archive/i-019-input-delivery-durability/index.md)) supplies
  replay-safe judgment handoff; Run completion
  ([i-020-run-completion](../i-020-run-completion/index.md)) supplies durable terminal facts.
  Machine composition follows those four contracts.
- The recursion-depth open question stays recorded and unresolved until a ruling answers it.
