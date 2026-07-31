# Issue design

## Proposed mechanics

No new mechanics are proposed here: every ask condenses a decision already recorded in the seed,
the doctrine-convergence evidence seed, at the 2026-07-29 split. This file summarizes only what
its three asks need and cites the seed's records and the research files; it never copies them.

- **Canonical residency and addressing (FDC-004).** Runs reside under the plural `runs` path with
  one id namespace; verdict slots nest under their run's directory, and a per-run spawn-ledger path
  is reserved there by name — the ledger's contract (contents, when written, meaning) is defined by
  the machine-composition issue ([i-018-machine-composition](../i-018-machine-composition/spec.md)),
  not here (scope corrected 2026-07-29, accepted review fix; old→new wording recorded in
  [spec.md](spec.md), "Scope correction — spawn ledger"). Addressing is `--run <id>`, always
  required, refusing with the roster when the value is missing. Decided as
  adopted defaults - `08` §4 items 6, 7, and 14, batch human sign-off, 2026-07-29 - recorded in the
  seed's Adopted defaults record
  ([design.md](../i-016-fsm-doctrine-convergence/design.md)). Research ground:
  [04-run-identity.md](../../../research/re-ratmac-FSM/04-run-identity.md) (run identity and the
  listing-is-the-registry property) and
  [05-invocation-join.md](../../../research/re-ratmac-FSM/05-invocation-join.md) (verdict slots and
  the spawn ledger).
- **Pin and residue (FDC-005).** The runbook pin stays hash-only; a flat-layout residue refuses and
  instructs, never auto-migrates. Decided as adopted defaults - `08` §4 items 8 and 9 - in the same
  record. Research ground:
  [06-migration-cost.md](../../../research/re-ratmac-FSM/06-migration-cost.md) (migration and
  sequencing facts about the current state).
- **Uncapped runs, never-reused ids (FDC-006).** No active-Run cap; within the one run-id namespace,
  ids are never reused after abandon; respawn mints a new id, a namespace fact (scope corrected
  2026-07-29, accepted review fix: what the ledger entry records about the superseded id is machine
  composition's contract, at the location this issue reserves — old→new wording recorded in
  [spec.md](spec.md), "Scope correction — spawn ledger"). Decided as
  adopted defaults - `08` §4 items 10 and 11, closing the cap-and-reuse gap (`AR-09`) - in the same
  record.

**Dependency — corrected 2026-07-29** (accepted review fix, Billy; three independent reviews),
superseding the paragraph that stood here. Old: "This stratum stands on the verdict-routed execution
core issue ([i-016-fsm-doctrine-convergence](../i-016-fsm-doctrine-convergence/index.md)): by the
edge-selection ruling (`AR-03`) verdict slots and per-run residency join the first increment
together, and the current engine routes first-edge-wins and cannot branch — residency layout has
nothing to serve until verdict-routed execution exists." New route (human ruling, 2026-07-29): run
residency (`i-017-run-residency`) lands first; the verdict-routed execution core
(`i-016-fsm-doctrine-convergence`) depends on it; machine composition (`i-018-machine-composition`)
depends on both. This issue depends on nothing else in the split — its layout, id namespace, and
refusal behavior are self-contained, proven against the research files cited above.

*Billy's 2026-07-30 cut* created input-routed transitions (`i-016-fsm-doctrine-convergence`),
input delivery and durability (`i-019-input-delivery-durability`), and Run completion
(`i-020-run-completion`). *Assumed dependency forecast, revocable at planning step 1:* input
delivery follows input-routed transitions; Run completion is routing-independent but builds on
this issue's addressed State File; machine composition follows routing, delivery, and completion.
This integrated residency issue still depends on nothing else.

This file is incoming evidence. Integrated mechanics remain authoritative only in the accepted forward authority.
