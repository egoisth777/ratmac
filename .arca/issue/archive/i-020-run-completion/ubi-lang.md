# Ubiquitous language

`FDC` expands to **FSM Doctrine Convergence**. The prefix was coined in the doctrine-convergence seed and remains unchanged across its split issues so requirement identifiers stay stable.

`FDCV` expands to **FSM Doctrine Convergence Verification**. The check prefix was coined in the evidence seed and remains stable across the split test plans.

## Terms

| Term | Meaning |
| :--- | :--- |
| Terminal state | A state with no ordinary outgoing edge. Entering it completes ordinary execution. The runbook schema calls a state a `Phase`. |
| Passed Run | A Run whose Engine-owned status is `passed` because it started in or advanced into a terminal state. |
| Abandoned event | The durable Engine-written history fact recorded before an explicitly abandoned Run's active state is retired. `abandoned` is not a surviving State File value. |
| Failed outcome | A deferred terminal outcome with no current Engine-observable trigger. Guard refusal is not failure. |
