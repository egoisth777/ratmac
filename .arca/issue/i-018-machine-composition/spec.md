# Issue specification

Each ask below states a settled position from the closed adversarial-review ledger of the
doctrine-convergence evidence seed ([test-plan.md](../i-016-fsm-doctrine-convergence/test-plan.md)),
written as a requirement candidate for P1 fold-in. Dispositions record the author's proposed decision;
P1 confirms or revises them at integration. Every rationale names its decision provenance: a **human
ruling** (not revocable by an agent) or an **adopted default** (revocable; batch human sign-off,
2026-07-29), both recorded in the seed's decision records
([design.md](../i-016-fsm-doctrine-convergence/design.md)).

Moved from the i-016 seed at the 2026-07-29 split.

`FDC` expands to **FSM Doctrine Convergence** - the requirement-ID prefix shared across the split,
defined in [ubi-lang.md](ubi-lang.md); IDs stay stable and are never renumbered.

## Requirement Records

| Requirement ID | Requirement | Disposition | Rationale | Accepted Forward Authority Refs |
| :--- | :--- | :--- | :--- | :--- |
| `FDC-007` | Authorization splits by motion kind: `spawn` is ordinary motion with no confirmation phrase; `respawn` and abandon-with-run-id require confirmation phrases naming the run id. | proposed | Exceptional motion needs a human; ordinary motion needs neither. Provenance: adopted default (`08` §4 item 15; revocable). | — (pending P1) |
| `FDC-008` | Termination is checked by guard-kind membership: every phase on a cycle carries at least one out-edge guarded by receipt- or contract-class guards only. | proposed | Monotonicity is not a property the guard vocabulary exposes; kind membership is, so the check is mechanical. Provenance: adopted default (`08` §4 item 13; revocable). | — (pending P1) |
| `FDC-009` | The runbook format is explicitly extended to carry the class and spawn tables `05` §1 introduces, superseding the format-spec restriction (`RBS-004`) the review cites; the canonical spelling is `blocked-route` (hyphen). | proposed | Without the claimed supersession the self-hosting runbook would refuse today; the hyphen spelling matches the parser, the specification, and the working rules. Provenance: human ruling (`AR-10`, 2026-07-29). | — (pending P1) |
| `FDC-010` | Judge independence lands in sequence: child-as-reviewer first; the witnessed verdict verb is deferred. | proposed | The witnessed verb needs signer identity, which `ORS-001` deliberately keeps out of the Engine. Provenance: adopted default (`08` §4 item 17; revocable). | — (pending P1) |

## Open question (not an ask)

- **Recursion depth** — may a child spawn grandchildren in the first increment, or is composition
  capped at one level until need shows? Unruled: no human ruling and no adopted default covers it.
  This issue records the fork and does not resolve it; the answer lands as a ruling, a goal
  requirement, or a new issue — never as an agent choice.

## Acceptance criteria

- Every requirement above traces to its decision record in the seed's
  [design.md](../i-016-fsm-doctrine-convergence/design.md) and names its provenance: human ruling or
  adopted default (revocable).
- **Billy's 2026-07-30 cut** created input-routed transitions, input delivery and durability, and
  Run completion as separate pending concerns. **Assumed dependency forecast, revocable at planning
  step 1:** integrated Run residency supplies addresses; input-routed transitions
  ([i-016-fsm-doctrine-convergence](../i-016-fsm-doctrine-convergence/index.md)) supply legal inputs
  and edge selection; input delivery
  ([i-019-input-delivery-durability](../i-019-input-delivery-durability/index.md)) supplies
  replay-safe judgment handoff; Run completion
  ([i-020-run-completion](../i-020-run-completion/index.md)) supplies durable terminal facts.
  Machine composition follows those four contracts.
- The recursion-depth open question stays recorded and unresolved until a ruling answers it.
