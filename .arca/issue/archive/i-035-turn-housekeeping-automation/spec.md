# Issue specification

Dispositions were confirmed at the 2026-08-24 planning pass (P1), signed by
Billy. The three boundaries the wish left open - the command boundary, where
the lifecycle lives, and who owns recovery when an interrupted close cannot
resume cleanly - were decided in the same pass and are recorded in the goal
design
([ADR-0018](../../../goal/design.md#the-turn-lifecycle-is-shop-tooling-beside-the-trial-lifecycle-adr-0018)).

`THK` is this issue's stable requirement-ID prefix — **Turn Housekeeping**
— defined in [ubi-lang.md](ubi-lang.md). The prefix was verified unique
against `.arca/goal/spec.md`, `.arca/schema.md`, and the whole repository at
filing.

## Requirement Records

| Requirement ID | Requirement | Disposition | Rationale | Accepted Forward Authority Refs |
| :--- | :--- | :--- | :--- | :--- |
| `THK-001` | One command opens a work item's turn: it creates the linked worktree on a branch named after the item, and copies the declared untracked lanes root into the worktree, skipping each lane's declared build output. Every precondition — clean trunk, no colliding branch or worktree registration, lanes root present — is checked before the first Git write; a failed precondition refuses with a named reason and zero mutation. The item is addressed as the opaque string a Run binding already carries (`PCR-007`), and the lanes root, trunk branch, and skip list are declared runbook data, so the mechanism learns no ticket concept (`NRR-001`). | accepted | Turn opening is re-derived by hand every turn — four times on 2026-08-21 alone ([log.md:433-436](../../../log.md)) — and the copy-in duty exists only as prose ([schema.md:127-129](../../../schema.md)). Declared data plus the opaque item address keeps the command Engine-generic; `TWL-001`'s check-before-first-write refusal is the proven shape. | [goal spec](../../../goal/spec.md#integrated-turn-housekeeping-requirements) |
| `THK-002` | One command closes a green turn in the working rules' fixed order, each step refusing on its own failure and blocking every later step: merge into the trunk (fast-forward when it has not moved, otherwise one merge commit that is itself a landing); copy the lanes root back to the primary checkout and verify the copy; record the landing's commit identity in the item record's declared landed-commit field; append the landing's line to the declared landing log; only then remove the worktree and delete the branch; finally re-run the lanes once from the trunk. An interrupted close resumes from the last completed step and never redoes a landed mutation. This mechanizes the prose duty ([schema.md:130-133](../../../schema.md), [schema.md:97](../../../schema.md)); because nothing is removed until copy-back has verifiably happened, copy-back can no longer be skipped. | accepted | The prose order has failed twice in practice — t-076 on 2026-08-06 ([log.md:310](../../../log.md)) and t-102 on 2026-08-21 — so a rule that must be remembered perfectly at end-of-turn is not a guard. The t-102..t-105 pattern (worktree add, lanes copy-in, then copy-back, removal, branch delete, landed-commit stamp, and log append at merge — `landed-commit: "6cee4bb"` at [t-102.md:26](../../../ticket/archive/t-102.md)) is exactly the sequence a close command owns. `TWL-005`'s fixed-order finish proves the shape. | [goal spec](../../../goal/spec.md#integrated-turn-housekeeping-requirements) |
| `THK-003` | A destructive removal — worktree removal, branch deletion, or any discard of the worktree's untracked content — refuses while the worktree holds the only copy of any artifact: an untracked file or directory that exists nowhere outside that worktree. The refusal names the artifact and the copy-back that would release it; no flag or force variant bypasses it, and the removal proceeds only after the artifact is verified present in the primary checkout or explicitly declared obsolete by its owner. | accepted | Both recorded losses ran through `git worktree remove --force` while the crate was "gitignored and committed nowhere" ([log.md:310](../../../log.md); [schema.md:131-132](../../../schema.md) names the exact mechanism). t-102's recovery by surviving authoring context is not a policy; a refusal at the only moment it can help — before the bytes go away — is. | [goal spec](../../../goal/spec.md#integrated-turn-housekeeping-requirements) |
| `THK-004` | The lifecycle offers a dry run that reports, for each mutating verb, the exact planned mutations and recovery commands while changing nothing observably (`TWL-006`'s discipline), and its ownership mirrors the trial lifecycle's (`TWL-010`): the verbs are invoked by the human or Main-Agent from the primary checkout; an Advisor authors content only; a Subagent invokes no lifecycle verb. A removal blocked by a held directory refuses by name with guidance — nothing forces a removal and nothing kills a process. | accepted | The wish names dry-run, rollback or resume, and clear ownership as the desired end. Resume is carried by `THK-002`; the dry run and ownership complete the lifecycle. The trial verbs already prove both on this repository ([schema.md:805-825](../../../schema.md#trial-worktrees), [tools/trial.ps1](../../../../tools/trial.ps1)). | [goal spec](../../../goal/spec.md#integrated-turn-housekeeping-requirements) |

## Acceptance criteria

- On a fixture repository, the open command produces a registered worktree
  on the item-named branch with the lanes root copied in (build output
  skipped), and each unsafe precondition refuses with a named reason and
  zero mutation of refs, index, worktree registrations, and both trees.
- The close command completes the fixed order end to end; a step forced to
  fail (an unmergable trunk, a copy-back that cannot verify) refuses at that
  step, leaves the completed steps inspectable, and a re-invocation resumes
  from the refusal without redoing a landed mutation.
- A removal attempted while the worktree holds an untracked artifact absent
  from the primary checkout refuses naming the artifact, and no flag variant
  bypasses the refusal; after a verified copy-back the same removal
  proceeds.
- The dry run prints planned mutations and recovery commands and leaves
  refs, index, working trees, and worktree registrations byte-identical.
- One real turn of this repository's cycle — open, work, close — is driven
  through the commands with no hand-run Git between the open and the
  completed close, and its landing carries the stamp, the log line, and the
  trunk lanes rerun.
