# Issue design

## Proposed mechanics

### Separate the record from its routing value

The judge-authored **live verdict record** is the durable handoff artifact. It contains one **transition input value** from the current state's legal list plus rationale. The Machine Class gives that value meaning by mapping it to one outgoing edge; the Engine validates and applies the mapping. The Engine does not choose the value.

This distinction does not silently rename accepted artifacts. The integrated run-residency goal defines `Verdict slot`, and the implementation currently reserves `verdict.toml` under each Run. Exact state-specific addressing and the record's TOML fields remain integration decisions. If a later decision changes that accepted path or term, it must update the goal, vocabulary, source, tests, and migration behavior explicitly.

### Consume before advance

For a valid live record:

1. Validate that its transition input belongs to the current state's legal input list.
2. Derive a collision-free evidence name from the state and the next on-disk attempt number.
3. Atomically rename the live verdict record into immutable evidence on the same filesystem. This one motion both archives the record and clears the live slot.
4. Write the successor State File only after the rename succeeds.

Interruption behavior is deliberate:

- before the rename: old state and live record remain;
- after the rename but before the State File write: old state remains, the record is archived, and a fresh verdict is required;
- after the State File write: the Run is advanced and the consumed record is already immutable.

No journal or recovery subsystem is added.

### Authorship boundary

A designated external judge—an agent or a human-mediated mechanism—authors the live record from evidence. This issue defines neither agent selection nor signer identity. A witnessed-verdict verb remains deferred because the Engine intentionally carries no caller identity; a `signed_by` field in an agent-writable verdict file would not prove human approval.

## Provenance

The consume ordering and atomic-rename refinement come from the human ruling for `AR-06` in the doctrine-convergence seed's [design](../i-016-fsm-doctrine-convergence/design.md). The seed remains the evidence history; this issue is the current requirement home.

This file is incoming evidence. Integrated mechanics remain authoritative only in the accepted forward authority.
