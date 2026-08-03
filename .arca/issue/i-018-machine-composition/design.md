# Issue design

## Proposed mechanics

No new mechanics are proposed here: every ask condenses a decision already recorded in the seed,
the doctrine-convergence evidence seed, at the 2026-07-29 split. This file summarizes only what
its four asks need and cites the seed's records and the research files; it never copies them.

- **Spawn authorization (FDC-007).** `spawn` is ordinary motion with no confirmation phrase;
  `respawn` and abandon-with-run-id require confirmation phrases naming the run id. Decided as an
  adopted default - `08` §4 item 15, closing the authorization gap (`AR-11`), batch human sign-off
  2026-07-29 - recorded in the seed's Adopted defaults record
  ([design.md](../archive/i-016-fsm-doctrine-convergence/design.md)).
- **Cycle termination (FDC-008).** Every phase on a cycle carries at least one out-edge guarded by
  receipt- or contract-class guards only, so termination is checked by guard-kind membership.
  Decided as an adopted default - `08` §4 item 13, closing the termination gap (`AR-07`) - in the
  same record.
- **Format extension (FDC-009).** The runbook format explicitly carries the class and spawn tables
  [05-invocation-join.md](../../research/re-ratmac-FSM/05-invocation-join.md) §1 introduces,
  superseding the format-spec restriction (`RBS-004`); the canonical spelling is `blocked-route`
  (hyphen). Decided by individual human ruling (`AR-10`, 2026-07-29), recorded in the seed's
  Individual human rulings record
  ([design.md](../archive/i-016-fsm-doctrine-convergence/design.md)).
- **Judge independence (FDC-010).** Child-as-reviewer lands first; the witnessed verdict verb is
  deferred - it needs signer identity, which `ORS-001` deliberately keeps out of the Engine.
  Decided as an adopted default - `08` §4 item 17 - in the same record. Research ground for the
  composition model: [05-invocation-join.md](../../research/re-ratmac-FSM/05-invocation-join.md)
  (spawn ledger, join) and
  [07-conceptual-model.md](../../research/re-ratmac-FSM/07-conceptual-model.md) (the composed
  machine picture, provenance-marked).

**Billy's 2026-07-30 cut** created three concerns below composition: input-routed transitions,
input delivery and durability, and Run completion. **Assumed dependency forecast, revocable at
planning step 1:** Run residency (`i-017-run-residency`) supplies per-Run addresses;
input-routed transitions (`i-016-fsm-doctrine-convergence`) then supply legal values and edge
selection; input delivery (`i-019-input-delivery-durability`) supplies replay-safe judgment
handoff; Run completion (`i-020-run-completion`) supplies durable terminal facts for joins.
Machine composition follows those four contracts.

**Spawn-ledger contract.** The ledger's contract - contents, when written, meaning - is defined
here, at the per-run location run residency reserves as a name only
([i-017-run-residency/spec.md](../archive/i-017-run-residency/spec.md), `FDC-004`). Added 2026-07-29
(accepted review fix): the contract includes the superseded-record entry - what a respawn records
about the id it supersedes - per FDC-006's narrowed remainder, reserved at the same location
([i-017-run-residency/spec.md](../archive/i-017-run-residency/spec.md), `FDC-006`). Research ground:
[05-invocation-join.md](../../research/re-ratmac-FSM/05-invocation-join.md) ("The spawn ledger").

This file is incoming evidence. Integrated mechanics remain authoritative only in the accepted forward authority.
