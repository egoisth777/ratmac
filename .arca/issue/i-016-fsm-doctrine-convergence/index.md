# Input-routed transitions

```yaml
issue-id: "i-016-fsm-doctrine-convergence"
provenance: "User request, 2026-07-29 - converge the wave-2 FSM/composition research; atomically cut by Billy on 2026-07-30"
status: "pending"
```

## Summary

This issue is the evidence seed of the 2026-07-29 doctrine-convergence work and the current pending home of one atomic concern: input-routed transitions (`FDC-001`). A branching state declares a closed legal input list, every ordinary outgoing edge maps exactly one unique value from that list, and the Engine selects the matching edge. Ordinary guards remain readiness checks, never destination selectors. Straight-line states may retain one unlabelled ordinary edge; blocked routes remain outside selection and completeness checks.

The current Engine still routes first-edge-wins and cannot branch on a typed input. This issue therefore owns the Machine Class declaration, parser representation, static doctor checks, and deterministic state-plus-input transition semantics as one vertical contract. Exact TOML spelling is not decided here; planning step 1 must place it in the runbook-format single source of truth ([runbook-spec.md](../../runbook-spec.md)).

The adversarial-review ledger in [test-plan.md](test-plan.md), its rulings in [design.md](design.md), and the research corrections remain here as shared evidence history. Split issues cite them and never copy them.

## Atomic split

Billy's 2026-07-30 cut preserved every requirement identifier and gave each concern one pending home:

- `FDC-001` — input-routed transitions: retained here.
- `FDC-003` — input delivery and durability: moved to [i-019-input-delivery-durability](../i-019-input-delivery-durability/index.md), which depends on this issue.
- `FDC-002` — Run completion: moved to [i-020-run-completion](../i-020-run-completion/index.md), independent of routing and delivery.

Witnessed or human-signed judgment remains deferred because the Engine carries no signer identity. Judge independence (`FDC-009` and `FDC-010`) remains solely in the machine-composition issue ([i-018-machine-composition](../i-018-machine-composition/index.md)). The accepted `Verdict slot` and `verdict.toml` names are not silently renamed by this issue cut.

Earlier split, identifiers unchanged: Run residency (`FDC-004` through `FDC-006`) lives in [i-017-run-residency](../i-017-run-residency/index.md); machine composition (`FDC-007` through `FDC-010`) lives in [i-018-machine-composition](../i-018-machine-composition/index.md). Assumed dependency forecast, revocable at planning step 1: Run residency precedes input-routed transitions, input delivery follows input-routed transitions, Run completion is routing-independent, and machine composition follows the contracts it consumes.

## Routes

| Need | File |
| :--- | :--- |
| Terms | [Ubiquitous language](ubi-lang.md) |
| Requirements | [Specification](spec.md) |
| Proposed mechanics and decision history | [Design](design.md) |
| Verification, adversarial-review ledger, and integration traces | [Test plan](test-plan.md) |

## Disposition log

- 2026-07-29: deferred at the planning pass because input-routed transitions awaited integrated Run residency; status stayed `pending`.
- 2026-07-30: Billy cut the pending issue into atomic concerns. This seed retained `FDC-001`; `FDC-003` moved to the input-delivery issue and `FDC-002` moved to the Run-completion issue, both under unchanged identifiers.
