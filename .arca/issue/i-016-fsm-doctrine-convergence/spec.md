# Issue specification

This issue remains the doctrine-convergence evidence seed and now owns one atomic requirement. Its [design](design.md) preserves all fourteen adversarial-review rulings and adopted defaults as history; moved requirements cite that history rather than copying it.

`FDC` expands to **FSM Doctrine Convergence**. The prefix remains shared across the split issues so existing requirement identifiers never change.

## Requirement Records

| Requirement ID | Requirement | Disposition | Rationale | Accepted Forward Authority Refs |
| :--- | :--- | :--- | :--- | :--- |
| `FDC-001` | Edge selection is input-only: ordinary guards are readiness AND-gates ("can we move"), never selectors ("where to"). A branching state declares a closed legal input list; every ordinary outgoing edge carries exactly one unique value from that list. Missing coverage, duplicate coverage, edge values outside the list, and mixed labelled and unlabelled ordinary edges are errors. A straight-line state may retain one unlabelled ordinary edge. Blocked routes stay outside selection and completeness checks. Given a current state and one legal transition input, the Engine selects exactly the one edge carrying that value. | proposed | Routing ambiguity is excluded by construction: a closed, completely covered input list leaves no edge without a defined value and no value without one destination. Provenance: human ruling for adversarial-review finding `AR-03` and Billy's 2026-07-30 clarification that the generic machine selector is the transition input extracted from a judge-authored verdict record; both are recorded in [design.md](design.md). | — (pending planning step 1) |

## Format boundary

This requirement changes the Machine Class format: the class must declare each branching state's legal input list and each ordinary outgoing edge's matching value. Exact TOML keys and diagnostic codes are not chosen by this pending issue. Planning step 1 must define the accepted spelling and checks in the runbook-format single source of truth ([runbook-spec.md](../../runbook-spec.md)); the parser, graph, doctor, scaffold, authoring repair table, and runtime must then implement that one definition together.

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
- The Machine Class format's accepted input-list and edge-label spelling has one definition in `.arca/runbook-spec.md`, with corresponding diagnostics and authoring repairs.
- Every legal transition input selects exactly one ordinary edge; malformed class shapes refuse before execution.
- The research files under `.arca/research/re-ratmac-FSM/` carry supersession notes wherever older text conflicts with this requirement.
