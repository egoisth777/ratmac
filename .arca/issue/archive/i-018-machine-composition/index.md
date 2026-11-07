# Machine composition

```yaml
issue-id: "i-018-machine-composition"
provenance: "Split of the FSM doctrine convergence issue (i-016) per human direction (Billy), 2026-07-29 - dependency strata; carries FDC-007 - FDC-010 from the i-016 seed unchanged, plus the recursion-depth open question"
status: "integrated"
```

## Summary

The top stratum of the 2026-07-29 dependency-strata split: machines spawning, judging, and
terminating each other. Four settled asks, IDs unchanged from the seed: `spawn` as ordinary motion
with `respawn` and abandon-with-run-id phrase-confirmed (FDC-007); cycle termination checked by
receipt- or contract-class guard-kind membership (FDC-008); the runbook format extended to carry
the class and spawn tables, superseding the format-spec restriction (`RBS-004`), with
`blocked-route` as the canonical spelling (FDC-009); child-as-reviewer first, the witnessed verdict
verb deferred (FDC-010). Two rulings of 2026-08-03 extend the asks: the spawn-ledger content
contract lives here as `FDC-011`, and composition is capped at one level as `FDC-012` — the
recursion-depth fork is resolved (see [spec.md](spec.md)).

**Billy's 2026-07-30 cut** created input-routed transitions, input delivery and durability, and
Run completion as separate pending concerns. **Assumed dependency forecast, revocable at planning
step 1:** integrated Run residency supplies per-Run addresses; input-routed transitions
([i-016-fsm-doctrine-convergence](../i-016-fsm-doctrine-convergence/index.md)) supply legal inputs
and deterministic edge selection; input delivery
([i-019-input-delivery-durability](../i-019-input-delivery-durability/index.md)) supplies
replay-safe judgment handoff; Run completion
([i-020-run-completion](../i-020-run-completion/index.md)) supplies durable terminal facts.
Machine composition follows those four contracts.

The decision records and the closed AR resolution ledger stay in the seed as evidence history;
this issue's files cite them, never copy.

## Routes

| Need | File |
| :--- | :--- |
| Terms | [Ubiquitous language](ubi-lang.md) |
| Requirements | [Specification](spec.md) |
| Proposed mechanics | [Design](design.md) |
| Verification and integration traces | [Test plan](test-plan.md) |

## Disposition log

- 2026-07-29: deferred at the 2026-07-29 planning pass (status stays `pending`) because it awaited
  integrated Run residency and the then-combined execution core.
- 2026-07-30: Billy's atomic cut refined the unmet dependencies to input-routed transitions,
  dependent input delivery and durability, and independent Run completion. Requirement identifiers
  stayed unchanged.
- 2026-07-30: all four asks were dispositioned `deferred`; the issue closes this batch as `integrated` with zero accepted goal rows. Steering retains machine composition in Horizon for a later issue after Run completion lands.

- 2026-07-31 correction: the preceding `integrated` conclusion is superseded. All four asks remain `deferred`, so the same five-file bundle stays live in the Deferred issue buffer with status `deferred`; no replacement issue carries them.

- 2026-08-03: Billy ruled both forks at selection: the spawn-ledger content contract is carried by
  this issue's spec as `FDC-011` (extending the 2026-07-29 scope settlement that named this issue
  the home), and recursion depth is capped at one level as `FDC-012` (a child Run may not itself
  spawn; lifting the cap is additive). The open question is resolved; the issue is selected now that
  Run completion (`FDC-002`) has landed: the same five-file bundle moved visibly to the intake work
  area with status `pending`, awaiting the P1 fold-in.

- 2026-08-03: all six asks dispositioned `accepted`; the requirements entered the goal
  (spec/design/test-list/ubi-lang/index) under "Integrated machine composition" and this bundle
  moved whole from intake to the archive. The Ideal-shape property advanced is Every boundary
  machine-checked - spawn authorization, cycle termination, join evaluation, and the depth cap
  are all Engine-checked boundaries - serving Self-hosted, because the cycle runbook delegates
  ticket turns through exactly this spawn/join surface.
