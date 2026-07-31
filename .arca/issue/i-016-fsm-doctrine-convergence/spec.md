# Issue specification

This issue remains the doctrine-convergence evidence seed and now owns one atomic requirement. Its [design](design.md) preserves all fourteen adversarial-review rulings and adopted defaults as history; moved requirements cite that history rather than copying it.

`FDC` expands to **FSM Doctrine Convergence**. The prefix remains shared across the split issues so existing requirement identifiers never change.

## Requirement Records

| Requirement ID | Requirement | Disposition | Rationale | Accepted Forward Authority Refs |
| :--- | :--- | :--- | :--- | :--- |
| `FDC-001` | Edge selection is input-only: ordinary guards are readiness AND-gates, never selectors. A branching Phase declares a closed legal input list; every ordinary outgoing transition carries exactly one unique value from that list, with complete coverage. Invalid lists, missing or duplicate coverage, foreign values, labelled blocked routes, and mixed labelled/unlabelled branches refuse. A straight-line Phase retains one unlabelled ordinary transition. Given the current Phase and one legal transition input, the Engine selects exactly the matching transition. | accepted | Human ruling on route selection, 2026-07-29, refined by Billy's 2026-07-30 input terminology and accepted at planning step 1 on 2026-07-30. | [goal `FDC-001`](../../goal/spec.md#integrated-input-routed-transition-requirements); mechanics in [goal design](../../goal/design.md#input-routed-transitions-fdc-001); [goal checks](../../goal/test-list.md#integrated-input-routed-transition-verification) |

## Format boundary

This requirement changes the Machine Class format. Planning step 1 accepts `inputs` on branching Phases and `input` on their ordinary outgoing transitions, with the checks fixed in the goal design. The implementation ticket must update `.arca/runbook-spec.md`, parser, graph, doctor, scaffold, authoring repair table, and runtime together so the executable and written diagnostic-code sets never diverge.

The accepted `Verdict slot` and current `verdict.toml` reservation are not renamed here. The input-delivery issue distinguishes the judge-authored record from the transition input it carries and records the cost of any later physical rename.

## Atomic split record (2026-07-30)

Billy split the earlier execution-core bundle by responsibility, preserving identifiers:

- `FDC-001` remains here: input lists, labelled edges, static validation, and state-plus-input edge selection.
- `FDC-003` moved to the input-delivery issue ([i-019-input-delivery-durability](../i-019-input-delivery-durability/spec.md)): external delivery plus atomic consume/archive before state advance. It depends on `FDC-001`.
- `FDC-002` moved to the Run-completion issue ([i-020-run-completion](../i-020-run-completion/spec.md)): Engine-written `passed`, durable abandonment, and explicit deferral of `failed`. It is independent of routing.
- Witnessed judgment remains deferred; judge independence remains solely under `FDC-009` and `FDC-010` in the machine-composition issue ([i-018-machine-composition](../i-018-machine-composition/spec.md)).

## Earlier split record (2026-07-29)

Run residency (`FDC-004` through `FDC-006`) moved to [i-017-run-residency](../i-017-run-residency/spec.md). Machine composition (`FDC-007` through `FDC-010`) moved to [i-018-machine-composition](../i-018-machine-composition/spec.md). The route remains residency before input-routed transitions, and both before machine composition.

## Acceptance criteria

- The adversarial-review ledger in [test-plan.md](test-plan.md) preserves all fourteen resolutions and links each resolution to its evidence.
- `FDC-001` traces to its human ruling in [design.md](design.md).
- The accepted `inputs`/`input` spelling and diagnostic semantics are fixed in goal design; the build updates `.arca/runbook-spec.md` and every executable/authoring consumer in one ticket.
- Every legal transition input selects exactly one ordinary edge; malformed class shapes refuse before execution.
- The research files under `.arca/research/re-ratmac-FSM/` carry supersession notes wherever older text conflicts with this requirement.
