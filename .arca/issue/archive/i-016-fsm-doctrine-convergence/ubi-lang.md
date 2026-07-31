# Ubiquitous language

`FDC` expands to **FSM Doctrine Convergence**. This evidence seed coined the prefix; all split issues reuse it so requirement identifiers stay stable.

`FDCV` expands to **FSM Doctrine Convergence Verification**. The check prefix was coined in this evidence seed and remains stable across the split test plans.

## Current terms

| Term | Meaning |
| :--- | :--- |
| Legal input list | The closed set of transition input values declared by the Machine Class for one branching state. Earlier issue text called this the verdict input list. |
| Transition input | One legal value used with the current state to select exactly one ordinary outgoing edge. It is the generic Engine-facing selector extracted from a judge-authored verdict record. |
| Input-only selection | Ordinary guards decide whether movement is ready; the transition input alone decides which ordinary edge is selected. |
| Straight-line state | A state with one ordinary outgoing edge. It may keep that edge unlabelled and needs no input list. |

## Moved terms

The following terms remain recorded here because this issue is the adversarial-review evidence seed, but their current requirement homes changed in Billy's 2026-07-30 cut:

| Term | Current home |
| :--- | :--- |
| Consume-then-advance, live verdict record, archived verdict | [Input delivery and durability](../i-019-input-delivery-durability/ubi-lang.md), requirement `FDC-003`. |
| Terminal state, Passed Run, abandoned event, failed outcome | [Run completion](../i-020-run-completion/ubi-lang.md), requirement `FDC-002`. |

The accepted `Verdict slot` term remains defined by the integrated goal and is not silently renamed by this issue cut. Judge independence remains in the machine-composition issue.
