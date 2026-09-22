# Issue specification

## Requirement Records

| Requirement ID | Requirement | Disposition | Rationale | Accepted Forward Authority Refs |
| :--- | :--- | :--- | :--- | :--- |
| `WCP-006` | The supported ticket damage and recovery mechanism resolves and records exactly one safety commit for the current reviewed green work, accepts only mutation targets tracked by that commit, and restores their index and worktree bytes only from that commit. Backup-copy, latest-file, implicit-index, arbitrary-revision, and inverse-edit restore modes are absent or explicitly refused before mutation. A stale checkpoint, untracked target, unsafe path, or unrelated unsaved work refuses by name. Interruption recovery uses the same recorded checkpoint and byte verifier. Historical evidence remains unchanged. | accepted | Accepted under Billy's 2026-09-22 all-wishes instruction and explicit workflow dispatch after independent contract review. | [Accepted authority](../../schema.md#wcp-006--checkpoint-only-restoration) |

## Evidence and boundaries

[Safety rules](../../schema.md#deliberate-damage-and-discard-safety) already require checkpoint restoration. [Turn lifecycle](../../../tools/turn.ps1) currently offers status, open, and close, with no dedicated checkpoint restoration boundary. This issue closes the mechanical gap rather than replacing the rule.

Existing files are observed evidence. Proposed checks below have not been executed and are not satisfaction evidence.

## Assumptions

Extend the contributor turn lifecycle with the narrow checkpoint/damage recovery capability rather than adding a second tool. The mechanism governs its own supported operations; it does not pretend to sandbox arbitrary shell commands. Private ignored lanes remain outside mutation targets, as the existing safety rule requires.

## Dependencies

Requires byte-verified restores; adopts the review-before-proofs snapshot as checkpoint provenance. Contributor-tools lane supplies ownership.

## Open decisions

No unresolved disposition remains. The requirement was accepted on 2026-09-22; the linked forward authority governs implementation and its adopted assumptions.
