# Issue test plan

## Verification

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| `THKV-001` | `THK-001` | On a fixture repository, `open` creates the item-named branch with its registered sibling worktree and the lanes root copied in, each lane's build output skipped. A dirty trunk, a colliding branch, a colliding worktree registration, and a missing lanes root each refuse with a named reason, and refs, index, both working trees, and worktree registrations are byte-identical before and after each refusal. |
| `THKV-002` | `THK-002` | On a fixture repository with a green turn open, `close` completes merge, verified copy-back, stamp, log line, removal, branch delete, and trunk rerun in order, each observable after the command. With one step forced to fail (an unmergable trunk; a copy-back whose verification cannot pass), the command refuses at that step, later steps do not run, and a re-invocation after repairing the cause resumes from the refusal without repeating a landed mutation — proven by the merge commit and log line counting exactly once. |
| `THKV-003` | `THK-003` | A worktree holding an untracked crate absent from the primary checkout refuses removal naming the crate and its copy-back; a force-flag variant of the same removal refuses identically; after a verified copy-back the removal proceeds. The t-076 shape — removal invoked before copy-back — is replayed as the red case: the guard refuses, and the crate survives. |
| `THKV-004` | `THK-004` | The dry run prints each mutating verb's planned mutations and recovery commands; refs, index, working trees, tags, and worktree registrations are byte-identical before and after. An invocation from inside the turn worktree refuses by name with the `cd` that fixes it; no code path kills a process or forces a removal. |

## Goal/Test File Traces

| Goal/Test File | Status | Reverse Issue Refs |
| :--- | :--- | :--- |
| `.arca/goal/index.md` | updated | Integrated turn housekeeping section at P1 |
| `.arca/goal/ubi-lang.md` | updated | Item turn, carried lanes, only-copy refusal, trunk |
| `.arca/goal/spec.md` | updated | THK-001..THK-004 |
| `.arca/goal/design.md` | updated | ADR-0018 records the boundary decisions |
| `.arca/goal/test-list.md` | updated | THKV-001..THKV-004 |

## Contributor Authority/Schema Traces

| Authority or Schema Artifact | Status | Integration and Reverse Refs |
| :--- | :--- | :--- |
| `AGENTS.md` | unaffected | - |
| `.arca/schema.md` | unaffected at integration | P1 resolved `THK-001`-`THK-004` as goal rows (`ADR-0018`); the Units-and-git prose revision and the trial-worktree ownership sibling land with the tickets that prove them |
| `.arca/wishlist.md` | carrier | The 2026-08-03 housekeeping wish stays until a landing fulfills it; this bundle is its intake carrier. |
| `.arca/steering.md` | updated | Current sprint regenerated at P1 close; this issue advances the Every-boundary-machine-checked and Self-hosted Ideal-shape properties |
