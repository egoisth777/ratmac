# Canonical run residency and identity

```yaml
issue-id: "i-017-run-residency"
provenance: "Split of the FSM doctrine convergence issue (i-016) per human direction (Billy), 2026-07-29 - dependency strata; carries FDC-004 - FDC-006 from the i-016 seed unchanged"
status: "integrated"
ideal-shape-property: "One writer, append-only"
```

## Summary

This issue lands first in the 2026-07-29 dependency-strata split: where runs live and how they are
named. Three settled asks, IDs unchanged from the seed: canonical residency under the plural `runs`
path with one id namespace and `--run <id>` always required (FDC-004); the hash-only runbook pin
with flat-layout residue refusing and instructing (FDC-005); uncapped runs with never-reused ids,
where respawn mints a new id, a namespace fact — what the ledger entry records about the superseded
id is the machine-composition issue's contract, at the location this issue reserves (FDC-006).

**Route corrected 2026-07-29** (accepted review fix, Billy; three independent reviews), superseding
the paragraph that stood here. Old: "this issue needs the verdict-routed execution core issue
([i-016-fsm-doctrine-convergence](../i-016-fsm-doctrine-convergence/index.md)) first — per-run
verdict slots and residency join the first increment together, and the current engine routes
first-edge-wins and cannot branch, so residency has nothing to serve until that core exists. The
machine-composition issue ([i-018-machine-composition](../i-018-machine-composition/index.md))
stands on this one in turn." New route (human ruling, 2026-07-29): run residency
(`i-017-run-residency`) lands first; the verdict-routed execution core
(`i-016-fsm-doctrine-convergence`) depends on it; machine composition (`i-018-machine-composition`)
depends on both. Rationale: verdict routing and consumption are stated in terms of the per-Run
verdict-slot address and Run-evidence location, and those contracts are defined by run residency;
requirements must not cite terms defined nowhere. This issue itself depends on nothing else in the split.

**Billy's 2026-07-30 cut** divided the historical execution core into input-routed transitions
(`i-016-fsm-doctrine-convergence`), input delivery and durability
(`i-019-input-delivery-durability`), and Run completion (`i-020-run-completion`).
**Assumed dependency forecast, revocable at planning step 1:** input delivery follows input-routed
transitions; Run completion is routing-independent and builds only on the addressed per-Run State
File; machine composition follows routing, delivery, and completion. This integrated residency
issue still depends on nothing else.

The decision records and the closed AR resolution ledger stay in the seed as evidence history;
this issue's files cite them, never copy.

## Routes

| Need | File |
| :--- | :--- |
| Terms | [Ubiquitous language](ubi-lang.md) |
| Requirements | [Specification](spec.md) |
| Proposed mechanics | [Design](design.md) |
| Verification and integration traces | [Test plan](test-plan.md) |

## P1 disposition — 2026-07-29

**Integrated.** All three asks were accepted at the 2026-07-29 planning pass (human batch sign-off,
Billy) and folded into the goal under their unchanged IDs — `FDC-004`, `FDC-005`, `FDC-006`:

| Goal file | Where the fold-in landed |
| :--- | :--- |
| Requirements | [spec.md — Integrated run-residency requirements](../../../goal/spec.md#integrated-run-residency-requirements) |
| Mechanics | [design.md — Canonical run residency and identity](../../../goal/design.md#canonical-run-residency-and-identity-fdc-004fdc-006) |
| Checks `RRV-001`–`RRV-004` | [test-list.md — Integrated run-residency verification](../../../goal/test-list.md#integrated-run-residency-verification) |
| Front door | [index.md — Integrated canonical run residency](../../../goal/index.md#integrated-canonical-run-residency) |

**Ideal-shape property advanced: One writer, append-only.** In steering's own words, "Run state has
exactly one writer and history only grows, so the record cannot be rewritten by whoever is working"
([steering.md, Ideal shape](../../../steering.md#ideal-shape)). Ids that are never reissued extend that
property from a Run's history to its address: an abandoned Run keeps its directory under the plural
`runs` path, and no later Run can occupy that address and overwrite the evidence. The two refusals
this issue states — a missing `--run <id>` answering with the roster, a flat-layout residue that
instructs instead of migrating — also serve *Refusals are branchable*, but the property that carries
this fold-in is the first.

**Sprint position.** This issue is the whole of the sprint regenerated at this P1 close
([steering.md, Current sprint](../../../steering.md#current-sprint)). The verdict-routed execution core
issue (`i-016-fsm-doctrine-convergence`), the machine-composition issue (`i-018-machine-composition`),
and the cycle-as-runbook issue (`i-015-cycle-as-runbook`) were deferred to the next planning pass per
the route ruling. In the goal, `FDC-004` and `FDC-006` supersede the v1 single-Run clauses `R-022` and
`R-023` — the lift ADR-0007 wrote as additive.

The 2026-07-30 cut later divided the deferred execution core into the input-routed-transition,
input-delivery, and Run-completion issues; this paragraph remains the stamped 2026-07-29 P1 record.
