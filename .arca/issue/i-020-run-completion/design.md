# Issue design

## Proposed mechanics

### Passing a Run

The Engine recognizes a terminal state structurally: it has no ordinary outgoing edge. Two entry paths apply the same rule:

1. `rtm start` creates a Run directly in a terminal initial state and writes status `passed`.
2. `rtm step` completes its transition into a terminal state and writes status `passed` with that successor state.

A terminal Run admits no further transition. The status belongs to the Engine and is never declared by the Machine Class.

### Abandoning a Run

Explicit, confirmed abandonment writes its durable terminal event before retiring active state and Run-scoped evidence. The event identifies the addressed Run and its last state. `abandoned` is an event in append-only history, never a surviving State File value. The existing all-or-none retirement behavior remains the safe implementation shape.

### Deferred failure

No current event objectively means `failed`. A guard refusal says only that movement is not presently authorized and must leave Run state unchanged. This issue therefore adds neither a failure command nor a `failed` write path. The later failed-outcome contract forecast in steering must name a concrete Engine-observable event before that outcome can exist.

## Provenance

The terminal vocabulary, Engine-written `passed`, durable abandonment, and deferred `failed` decisions come from the human rulings for `AR-04` and `AR-05` in the doctrine-convergence seed's [design](../i-016-fsm-doctrine-convergence/design.md). The seed remains the evidence history; this issue is the current requirement home.

This file is incoming evidence. Integrated mechanics remain authoritative only in the accepted forward authority.
