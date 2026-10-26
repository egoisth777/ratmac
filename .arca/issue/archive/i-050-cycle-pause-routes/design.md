# Issue design

## Proposed mechanics

- Use a blocked self-route for each working State so the graph position survives the hold; keep blocked as a lifecycle status rather than introducing a State called blocked.

- Reuse the existing confirmed hold and opaque blocker rules. A shop annotation on an affected ticket remains a contributor action, never an Engine write.

- Add one addressed, explicit resume operation whose exact human resolution authorization names both the Run and its currently recorded blocker. Under a governed policy, require a signed resolution decision through WRA-001 bound to those same facts and current trusted identities. The human or authorized reviewer owns the external resolution judgment; the Engine verifies the authorization and binding, not the truth of an external cause. A generic claim, an edited issue status, or deletion of the referenced file supplies no authorization.

- Reuse existing entry-prerequisite checks. Do not introduce a new guard language or issue parser to infer blocker resolution. The signed resolution purpose is exactly `ratmac-resume-v1`. Its canonical intent binds the Run identifier, current State, owning class, recorded workspace, effective pinned authority identities, exact opaque blocker, held-record digest, and hold occurrence. The human confirmation binds the same Run and blocker; under a governed policy it does not replace the signed decision.

- Each committed hold receives a monotonically increasing, never-reused occurrence in the addressed Run's Engine-owned append-only `pauses/<sequence>.toml`. Sequence filenames are positive decimal integers without leading zeroes, starting at 1 and ordered numerically; an unavailable next value refuses instead of wrapping. The record preserves the exact held context and its digest. Resuming and then holding the same Run at the same State against the same blocker creates a new occurrence, so a prior signature cannot resume it. No field is added to the public seven-field Run Record.

- Resume revalidates the owning class and pinned authority, takes the addressed Run lock, and compares the held State, exact blocker, record digest, and occurrence against the authorized request. Only after resolution authorization and existing entry prerequisites pass does it clear the live blocker and set executing status at the same State. Preserve the hold, blocker, and verified resolution authorization as historical facts without claiming the Engine proved an external repair. Never consume a verdict, mint a child, refresh a receipt, or advance a State during resume; ordinary operations do those jobs afterward under their existing checks.

- Hold and resume use an Engine-owned operation journal under the addressed Run lock. Prepare the complete old/new Run Record bytes, pause or resolution evidence, and uniquely identified append-only history event before publication. Atomically publish the complete journal entry as the action's commit decision; unpublished temporary preparation is not an accepted action. Before any later mutation, recover an interrupted operation to its unchanged old state when no commit decision exists, or finish all outputs of the committed action exactly once. Readers never repair: while physical outputs need reconciliation, they report the pending operation instead of presenting a half-cleared pause as ordinary state. A refusal before publication leaves persisted records and history unchanged; failure after publication reports a committed action with recovery pending, never a false unchanged refusal. Committed pause records and history are never removed or rewritten during recovery.

- Update the working procedure before activating the Runbook changes. Stage reviewed changes until existing pinned Runs reach a safe boundary; a current Run cannot inherit new routes by silent mutation. A rule-fix amendment preserves the paused lifecycle; it is not implicit resume permission and supplies no passing transition evidence.

## Scope and assumptions

These mechanics were selected during integration under Billy's 2026-09-22 dispatch. Implementation remains ticketed and must preserve runtime ownership and historical evidence; this issue claims no completed implementation.

Depends on child-class hold for ticket-stage pauses and WRA-001 for signed resolution authorization under a governed policy. Rule-fix recovery is separate and required only when continuing after a pinned rule itself changes. Its authority amendment and this capability's resume authorization remain separate decisions.

This file is incoming evidence. Integrated mechanics remain authoritative only in the accepted forward authority.
