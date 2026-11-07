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
| `FDC-004` | Runs reside canonically under the plural `runs` path with a single id namespace; verdict slots nest under their run's directory, and a per-run spawn-ledger path is reserved there by name (its contract — contents, when written, meaning — is defined by the machine-composition issue, [i-018-machine-composition](../i-018-machine-composition/spec.md), not here); run addressing is `--run <id>`, always required, and a missing value refuses with the roster. | accepted | Restores "the listing is the registry", removes the namespace collision, gives verdict addressing a computable base, and keeps recorded transcripts self-describing for behavioral evidence. Provenance: adopted defaults (`08` §4 items 6, 7, and 14; revocable). | [goal `FDC-004`](../../../goal/spec.md#integrated-run-residency-requirements); mechanics in [goal design](../../../goal/design.md#canonical-run-residency-and-identity-fdc-004fdc-006); checks `RRV-002` in [goal test list](../../../goal/test-list.md#integrated-run-residency-verification) |
| `FDC-005` | The runbook pin stays hash-only — no per-run copy without a demonstrated drift case — and a flat-layout residue refuses and instructs, never auto-migrates. | accepted | A per-run copy creates two files that can disagree; refusing without modifying follows the existing lock-refusal precedent. Provenance: adopted defaults (`08` §4 items 8 and 9; revocable). | [goal `FDC-005`](../../../goal/spec.md#integrated-run-residency-requirements); mechanics in [goal design](../../../goal/design.md#canonical-run-residency-and-identity-fdc-004fdc-006); checks `RRV-003` in [goal test list](../../../goal/test-list.md#integrated-run-residency-verification) |
| `FDC-006` | Multi-run is uncapped: no active-Run cap; within the one run-id namespace, ids are never reused after abandon; respawn mints a new id, a namespace fact (the `respawn` verb itself belongs to the machine-composition issue, [i-018-machine-composition](../i-018-machine-composition/spec.md)) — what the ledger entry records about the superseded id is that issue's contract, at the location this issue reserves. | accepted | Any cap below the fan-out width refuses mid-spawn and makes the child bundle unusable; never-reused ids preserve failure evidence and unforgeable addresses. Provenance: adopted defaults (`08` §4 items 10 and 11; revocable). | [goal `FDC-006`](../../../goal/spec.md#integrated-run-residency-requirements); mechanics in [goal design](../../../goal/design.md#canonical-run-residency-and-identity-fdc-004fdc-006); checks `RRV-004` in [goal test list](../../../goal/test-list.md#integrated-run-residency-verification) |

### Scope correction — spawn ledger (2026-07-29, accepted review fix)

FDC-004's requirement text is amended to narrow scope; the ID and the rest of the ask stay
unchanged. Old: "verdict slots and spawn ledgers nest under their run's directory". New (reflected
in the table above): "verdict slots nest under their run's directory, and a per-run spawn-ledger
path is reserved there by name; the ledger's contract — contents, when written, meaning — is
defined by the machine-composition issue
([i-018-machine-composition](../i-018-machine-composition/spec.md)), not here." Rationale:
spawn-ledger semantics belong to machine composition (i-018), not run residency (i-017);
requirements must not cite a contract defined nowhere in this issue's scope. This issue's scope
stays minimal: residency layout, one run-id namespace, `--run <id>` required, and refuse-and-instruct
for the old flat form (FDC-005).

FDC-006's requirement text is amended the same way, same date. What stays: run ids are never reused,
within the one run-id namespace; respawn mints a new id, a namespace fact. What moves: the
ledger-entry claim is ledger content — the entry's shape and when it is written are machine
composition's contract, not run residency's. Old: "run ids are never reused after abandon; respawn
mints a new id and the spawn ledger entry records the superseded one." New (reflected in the table
above): "within the one run-id namespace, ids are never reused after abandon; respawn mints a new
id, a namespace fact (the `respawn` verb itself belongs to the machine-composition issue) — what
the ledger entry records about the superseded id is that issue's contract, at the location this
issue reserves." Rationale: same as FDC-004 — ledger content is machine composition's to define, not
run residency's.

## Acceptance criteria

- Every requirement above traces to its decision record in the seed's
  [design.md](../i-016-fsm-doctrine-convergence/design.md) and names its provenance: human ruling or
  adopted default (revocable).
- **Route (human ruling, 2026-07-29)**, superseding an earlier criterion here that read "the
  dependency holds: the verdict-routed execution core issue lands before this one — per-run verdict
  slots and residency join the first increment together": run residency (`i-017-run-residency`)
  lands first; the verdict-routed execution core (`i-016-fsm-doctrine-convergence`) depends on it;
  machine composition (`i-018-machine-composition`) depends on both. This issue depends on nothing
  else in the split.

- **Billy's 2026-07-30 cut** created input-routed transitions
  (`i-016-fsm-doctrine-convergence`), input delivery and durability
  (`i-019-input-delivery-durability`), and Run completion (`i-020-run-completion`).
  **Assumed dependency forecast, revocable at planning step 1:** input delivery follows
  input-routed transitions; Run completion is routing-independent and builds on this issue's
  addressed State File; machine composition follows routing, delivery, and completion. This
  integrated issue remains dependency-free.

## P1 disposition — folded in at the 2026-07-29 planning pass

All three asks were **accepted** at P1 close (human batch sign-off, Billy, 2026-07-29) and now stand in the
forward authority under their unchanged IDs: `FDC-004`, `FDC-005`, and `FDC-006` in
[goal spec.md](../../../goal/spec.md#integrated-run-residency-requirements), their mechanics in
[goal design.md](../../../goal/design.md#canonical-run-residency-and-identity-fdc-004fdc-006), their checks
`RRV-001`–`RRV-004` in [goal test-list.md](../../../goal/test-list.md#integrated-run-residency-verification),
and the routing entry in [goal index.md](../../../goal/index.md#integrated-canonical-run-residency). The
preamble sentence above — dispositions as the author's proposal — is superseded for this issue: the
Disposition column now records the P1 decision itself. The goal wording matches the narrowed text of this
file, both 2026-07-29 scope corrections included. Ideal-shape property advanced: **One writer,
append-only** — an address never reissued keeps a finished Run's record from being overwritten.
In the goal, `FDC-004` and `FDC-006` supersede the v1 single-Run clauses `R-022` and `R-023`.
