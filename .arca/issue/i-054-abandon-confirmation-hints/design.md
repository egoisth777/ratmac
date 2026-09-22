# Issue design

## Proposed mechanics

- Keep `src/abandon.rs::required_phrase` as the phrase authority and make diagnostics use validated request context. Parse addressing before generating phrase-specific option hints; do not invent an address from a single-entry roster.

- Update module examples, command diagnostics, and working instructions together at implementation. Preserve the existing no-live-Run leftover-lock behavior and residue preflight precedence.

- No new confirmation source, bypass flag, or automatic approval is introduced. Displaying the proper command is guidance, not evidence that a human authorized it.

## Scope and assumptions

These are minimal proposed mechanics under Billy's 2026-09-22 instruction to proceed without questions. They authorize no source edit, runtime mutation, historical rewrite, or completed-work claim in this issue-creation stage.

Independent. Reuse the descriptive roster renderer if it lands first, without requiring that presentation change to fix the phrase.

This file is incoming evidence. Integrated mechanics remain authoritative only in the accepted forward authority.

